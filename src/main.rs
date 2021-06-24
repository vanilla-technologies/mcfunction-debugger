mod parser;
mod template_engine;

use crate::{
    parser::{parse_line, Line},
    template_engine::TemplateEngine,
};
use clap::{App, Arg};
use const_format::concatcp;
use futures::{future::try_join_all, FutureExt};
use load_file::load_str;
use multimap::MultiMap;
use parser::commands::{CommandParser, NamespacedName};
use std::{
    collections::HashMap,
    fs::{create_dir_all, write, File},
    io::{self, BufRead, Error},
    iter::{repeat, FromIterator},
    path::{Path, PathBuf},
};
use tokio::task::JoinHandle;
use walkdir::WalkDir;

const DATAPACK: &str = "datapack";
const OUTPUT: &str = "output";

const NAMESPACE: &str = "namespace";

#[tokio::main]
async fn main() -> io::Result<()> {
    let matches = App::new("mcfunction-debugger")
        .arg(
            Arg::with_name(DATAPACK)
                .long("datapack")
                .value_name("DIRECTORY")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(OUTPUT)
                .long(OUTPUT)
                .value_name("DIRECTORY")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(NAMESPACE)
                .long("namespace")
                .value_name("STRING")
                .takes_value(true)
                .required(true),
        )
        .get_matches();
    let datapack_path = Path::new(matches.value_of(DATAPACK).unwrap());
    let pack_mcmeta_path = datapack_path.join("pack.mcmeta");
    assert!(pack_mcmeta_path.is_file(), "Could not find pack.mcmeta");
    let output_path = Path::new(matches.value_of(OUTPUT).unwrap());
    let namespace = matches.value_of(NAMESPACE).unwrap();

    let parser =
        CommandParser::default().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let functions = find_function_files(datapack_path).await?;
    let function_contents = functions
        .iter()
        .map(|(name, path)| {
            //TODO async
            let file = File::open(path)?;
            let mut lines = io::BufReader::new(file)
                .lines()
                .enumerate()
                .map(|(line_number, line)| {
                    line.map(|line| {
                        let command = parse_line(&parser, &line);
                        (line_number + 1, line, command)
                    })
                })
                .collect::<io::Result<Vec<(usize, String, Line)>>>()?;

            // TODO dirty hack for when the last line in a file is a function call or breakpoint
            lines.push((lines.len() + 1, "".to_string(), Line::OtherCommand));
            Ok((name, lines))
        })
        .collect::<Result<HashMap<&NamespacedName, Vec<(usize, String, Line)>>, io::Error>>()?;

    generate_output_datapack(output_path, &function_contents, namespace)?;
    Ok(())
}

async fn find_function_files(
    datapack_path: &Path,
) -> Result<HashMap<NamespacedName, PathBuf>, io::Error> {
    let data_path = datapack_path.join("data");
    let threads = data_path
        .read_dir()?
        .collect::<io::Result<Vec<_>>>()?
        .into_iter()
        .map(|entry| get_functions(entry).map(|result| result?));

    Ok(try_join_all(threads)
        .await?
        .into_iter()
        .flat_map(|it| it)
        .collect::<HashMap<NamespacedName, PathBuf>>())
}

fn get_functions(
    entry: std::fs::DirEntry,
) -> JoinHandle<Result<Vec<(NamespacedName, PathBuf)>, io::Error>> {
    tokio::spawn(async move {
        let mut functions = Vec::new();
        if entry.file_type()?.is_dir() {
            let namespace = entry.file_name();
            let namespace_path = entry.path();
            let functions_path = namespace_path.join("functions");
            if functions_path.is_dir() {
                for f_entry in WalkDir::new(&functions_path) {
                    let f_entry = f_entry?;
                    let path = f_entry.path().to_owned();
                    let file_type = f_entry.file_type();
                    if file_type.is_file() {
                        if let Some(extension) = path.extension() {
                            if extension == "mcfunction" {
                                let relative_path = path.strip_prefix(&functions_path).unwrap();
                                let name = NamespacedName::new(
                                    namespace.to_string_lossy().as_ref(),
                                    &relative_path.with_extension("").display().to_string(),
                                );

                                functions.push((name, path));
                            }
                        }
                    }
                }
            }
        }
        Ok(functions)
    })
}

fn generate_output_datapack(
    output_path: &Path,
    function_contents: &HashMap<
        &parser::commands::NamespacedNameRef<String>,
        Vec<(usize, String, Line)>,
    >,
    namespace: &str,
) -> io::Result<()> {
    const RESOURCE_PATH: &str = "datapack_template/";
    const FN_PATH: &str = "data/-ns-/functions/";
    const PREFIX: &str = concatcp!(RESOURCE_PATH, FN_PATH);

    let engine = TemplateEngine {
        replacements: HashMap::from_iter(vec![("-ns-", namespace)]),
    };

    let fn_path = output_path.join(engine.expand(FN_PATH));

    create_dir_all(fn_path.join("id"))?;

    const ID_ASSIGN: &str = "id/assign.mcfunction";
    let content = engine.expand(load_str!(concatcp!(PREFIX, ID_ASSIGN)));
    write(fn_path.join(ID_ASSIGN), &content)?;

    const ID_INIT_SELF: &str = "id/init_self.mcfunction";
    let content = engine.expand(load_str!(concatcp!(PREFIX, ID_INIT_SELF)));
    write(fn_path.join(ID_INIT_SELF), &content)?;

    const ID_INSTALL: &str = "id/install.mcfunction";
    let content = engine.expand(load_str!(concatcp!(PREFIX, ID_INSTALL)));
    write(fn_path.join(ID_INSTALL), &content)?;

    const ID_UNINSTALL: &str = "id/uninstall.mcfunction";
    let content = engine.expand(load_str!(concatcp!(PREFIX, ID_UNINSTALL)));
    write(fn_path.join(ID_UNINSTALL), &content)?;

    create_continue_aec_file(&fn_path, function_contents, &engine)?;

    const CONTINUE: &str = "continue.mcfunction";
    let content = engine.expand(load_str!(concatcp!(PREFIX, CONTINUE)));
    write(fn_path.join(CONTINUE), &content)?;

    const INSTALL: &str = "install.mcfunction";
    let content = engine.expand(load_str!(concatcp!(PREFIX, INSTALL)));
    write(fn_path.join(INSTALL), &content)?;

    const SELECT_ENTITY: &str = "select_entity.mcfunction";
    let content = engine.expand(load_str!(concatcp!(PREFIX, SELECT_ENTITY)));
    write(fn_path.join(SELECT_ENTITY), &content)?;

    const UNINSTALL: &str = "uninstall.mcfunction";
    let content = engine.expand(load_str!(concatcp!(PREFIX, UNINSTALL)));
    write(fn_path.join(UNINSTALL), &content)?;

    const PACK: &str = "pack.mcmeta";
    let content = engine.expand(load_str!(concatcp!(RESOURCE_PATH, PACK)));
    write(output_path.join(PACK), &content)?;

    let call_tree = create_call_tree(&function_contents);

    for (name, lines) in function_contents.iter() {
        //TODO async
        create_function_files(&fn_path, name, lines, &call_tree, &engine, namespace)?;
    }

    Ok(())
}

fn create_continue_aec_file(
    output_function_path: &PathBuf,
    function_contents: &HashMap<&NamespacedName, Vec<(usize, String, Line)>>,
    engine: &TemplateEngine,
) -> io::Result<()> {
    let continue_cases = function_contents
        .iter()
        .flat_map(|(name, lines)| {
            repeat(name).zip(
                lines
                    .iter()
                    .filter(|(_, _, command)| matches!(command, Line::Breakpoint))
                    .map(|it| it.0),
            )
        })
        .map(|(name, line_number)| {
           engine.expand( &format!(
                "execute \
                  store success score continue_success -ns-_global \
                  if entity @s[tag=-ns-_{original_namespace}_{original_function_tag}_{line_number}] \
                  run function -ns-:{original_namespace}/{original_function}/{line_number_1}_continue",
                original_namespace = name.namespace(),
                original_function = name.name(),
                original_function_tag = name.name().replace("/", "_"),
                line_number = line_number ,
                line_number_1 = line_number + 1
            ))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let content = engine
        .expand(include_str!(
            "datapack_template/data/-ns-/functions/continue_aec.mcfunction"
        ))
        .replace("# -continue_cases-", &continue_cases);

    let path = output_function_path.join("continue_aec.mcfunction");
    write(&path, &content)?;

    Ok(())
}

fn create_call_tree<'l>(
    function_contents: &'l HashMap<&NamespacedName, Vec<(usize, String, Line)>>,
) -> MultiMap<&'l NamespacedName, (&'l NamespacedName, &'l usize)> {
    function_contents
        .iter()
        .flat_map(|(&caller, lines)| {
            lines
                .iter()
                .filter_map(move |(line_number, _line, command)| {
                    if let Line::FunctionCall { name: callee, .. } = command {
                        Some((callee, (caller, line_number)))
                    } else {
                        None
                    }
                })
        })
        .collect()
}

fn create_function_files(
    output_function_path: &PathBuf,
    name: &NamespacedName,
    lines: &Vec<(usize, String, Line)>,
    call_tree: &MultiMap<&NamespacedName, (&NamespacedName, &usize)>,
    engine: &TemplateEngine,
    namespace: &str,
) -> Result<(), Error> {
    let original_namespace = name.namespace();
    let original_function = name.name();
    let function_directory = output_function_path
        .join(original_namespace)
        .join(original_function);
    create_dir_all(&function_directory)?;

    let mut start_line = 1;
    for partition in lines.split_inclusive(|(_, _, command)| *command != Line::OtherCommand) {
        let first = start_line == 1;
        let end_line = start_line + partition.len();
        let line_numbers = format!("{}-{}", start_line, end_line - 1);

        let orig_fn_tag = original_function.replace('/', "_");
        let engine = engine.extend(vec![
            ("-orig_ns-", original_namespace),
            ("-line_numbers-", &line_numbers),
            ("-orig_fn-", &orig_fn_tag),
            ("-orig/fn-", original_function),
        ]);

        if first {
            let path = function_directory.join("iterate.mcfunction");
            let template = include_str!(
                "datapack_template/data/-ns-/functions/-orig_ns-/-orig/fn-/iterate.mcfunction"
            );
            write(&path, &engine.expand(template))?;

            let path = function_directory.join("iteration_step.mcfunction");
            let template  =include_str!(
                "datapack_template/data/-ns-/functions/-orig_ns-/-orig/fn-/iteration_step.mcfunction"
            );
            write(&path, &engine.expand(template))?;

            let path = function_directory.join("iterate_same_executor.mcfunction");
            let template = include_str!(
                "datapack_template/data/-ns-/functions/-orig_ns-/-orig/fn-/iterate_same_executor.mcfunction"
            );
            write(&path, &engine.expand(template))?;

            let path = function_directory.join("start.mcfunction");
            let template = include_str!(
                "datapack_template/data/-ns-/functions/-orig_ns-/-orig/fn-/start.mcfunction"
            );
            write(&path, &engine.expand(template))?;
        } else {
            let file_name = format!("{}_continue.mcfunction", start_line);
            let path = function_directory.join(file_name);
            let mut template = include_str!(
                "datapack_template/data/-ns-/functions/-orig_ns-/-orig/fn-/continue.mcfunction"
            )
            .to_string();
            if let Some(_callers) = call_tree.get_vec(name) {
                template.push_str(include_str!(
                    "datapack_template/data/-ns-/functions/-orig_ns-/-orig/fn-/continue_return.mcfunction"
                ));
            }
            write(&path, &engine.expand(&template))?;
        }
        start_line = end_line;

        // line_names.mcfunction
        let file_name = format!("{}.mcfunction", &line_numbers);
        let path = function_directory.join(file_name);
        let content = partition
            .iter()
            .map(|line| engine.expand_line(line, namespace))
            .collect::<Vec<_>>()
            .join("\n");
        let template = include_str!(
            "datapack_template/data/-ns-/functions/-orig_ns-/-orig/fn-/-line_numbers-.mcfunction"
        );
        let template = template.replace("# -content-", &content);
        write(&path, &engine.expand(&template))?;

        // line_numbers_with_context.mcfunction
        let file_name = format!("{}_with_context.mcfunction", &line_numbers);
        let path = function_directory.join(file_name);
        let template  = include_str!(
            "datapack_template/data/-ns-/functions/-orig_ns-/-orig/fn-/-line_numbers-_with_context.mcfunction"
        );
        write(&path, &engine.expand(template))?;
    }

    if let Some(callers) = call_tree.get_vec(name) {
        let return_cases = callers
            .iter()
            .map(|(caller, line_number)| {
                engine.expand(&format!(
                    "execute if entity @s[tag=-ns-_{caller_namespace}_{caller_function_tag}] run \
                     function -ns-:{caller_namespace}/{caller_function}/{line_number}_continue",
                    caller_namespace = caller.namespace(),
                    caller_function = caller.name(),
                    caller_function_tag = caller.name().replace("/", "_"),
                    line_number = *line_number + 1
                ))
            })
            .collect::<Vec<_>>()
            .join("\n");

        let content = engine
            .expand(include_str!(
                "datapack_template/data/-ns-/functions/-orig_ns-/-orig/fn-/return.mcfunction"
            ))
            .replace("# -return_cases-", &return_cases);

        let path = function_directory.join("return.mcfunction");
        write(&path, &content)?;
    }

    Ok(())
}
