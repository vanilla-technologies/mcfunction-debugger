mod parser;
mod template_engine;

use crate::{
    parser::{parse_line, Line},
    template_engine::TemplateEngine,
};
use clap::{App, Arg};
use futures::{future::try_join_all, FutureExt};
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
                .required(true)
                .validator(|namespace| {
                    if namespace.len() < 10 {
                        //max len of identifiers 16 => scoreboard {}_global has 7 characters -> 9 remaining for namespace
                        return Ok(());
                    }
                    Err(String::from("Max 'namespace' name length: 9 characters"))
                }),
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
    macro_rules! expand_template {
        ($e:ident, $p:literal) => {
            (
                $e.expand($p),
                $e.expand(include_str!(concat!("datapack_template/", $p))),
            )
        };
    }

    let engine = TemplateEngine::new(HashMap::from_iter(vec![("-ns-", namespace)]));

    create_dir_all(output_path.join(engine.expand("data/-ns-/functions/id")))?;

    let (path, content) = expand_template!(engine, "data/-ns-/functions/id/assign.mcfunction");
    write(output_path.join(path), &content)?;

    let (path, content) = expand_template!(engine, "data/-ns-/functions/id/init_self.mcfunction");
    write(output_path.join(path), &content)?;

    let (path, content) = expand_template!(engine, "data/-ns-/functions/id/install.mcfunction");
    write(output_path.join(path), &content)?;

    let (path, content) = expand_template!(engine, "data/-ns-/functions/id/uninstall.mcfunction");
    write(output_path.join(path), &content)?;

    create_continue_aec_file(&output_path, function_contents, &engine)?;

    let (path, content) = expand_template!(engine, "data/-ns-/functions/continue.mcfunction");
    write(output_path.join(path), &content)?;

    let (path, content) = expand_template!(engine, "data/-ns-/functions/decrement_age.mcfunction");
    write(output_path.join(path), &content)?;

    let (path, content) = expand_template!(engine, "data/-ns-/functions/install.mcfunction");
    write(output_path.join(path), &content)?;

    create_schedule_file(&output_path, function_contents, &engine)?;

    let (path, content) = expand_template!(engine, "data/-ns-/functions/select_entity.mcfunction");
    write(output_path.join(path), &content)?;

    let (path, content) = expand_template!(engine, "data/-ns-/functions/tick_start.mcfunction");
    write(output_path.join(path), &content)?;

    let (path, content) = expand_template!(engine, "data/-ns-/functions/tick.mcfunction");
    write(output_path.join(path), &content)?;

    let (path, content) = expand_template!(engine, "data/-ns-/functions/uninstall.mcfunction");
    write(output_path.join(path), &content)?;

    create_dir_all(output_path.join("data/minecraft/tags/functions"))?;

    let (path, content) = expand_template!(engine, "data/minecraft/tags/functions/tick.json");
    write(output_path.join(path), &content)?;

    let (path, content) = expand_template!(engine, "pack.mcmeta");
    write(output_path.join(path), &content)?;

    let call_tree = create_call_tree(&function_contents);

    let fn_path = output_path.join(engine.expand("data/-ns-/functions"));

    for (name, lines) in function_contents.iter() {
        //TODO async
        create_function_files(&fn_path, name, lines, &call_tree, &engine, namespace)?;
    }

    Ok(())
}

fn create_schedule_file<P: AsRef<Path>>(
    output_path: P,
    function_contents: &HashMap<&NamespacedName, Vec<(usize, String, Line)>>,
    engine: &TemplateEngine,
) -> io::Result<()> {
    #[rustfmt::skip]
    macro_rules! PATH { () => { "data/-ns-/functions/schedule.mcfunction" }; }

    let content = function_contents
        .keys()
        .map(|name| {
            let engine = engine.extend_orig_name(name);
            engine.expand(include_str!(concat!("datapack_template/", PATH!())))
        })
        .collect::<Vec<_>>()
        .join("");

    let path = output_path.as_ref().join(engine.expand(PATH!()));
    write(&path, &content)?;

    Ok(())
}

fn create_continue_aec_file<P: AsRef<Path>>(
    output_path: P,
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

    #[rustfmt::skip]
    macro_rules! PATH { () => { "data/-ns-/functions/continue_aec.mcfunction" }; }

    let content = engine
        .expand(include_str!(concat!("datapack_template/", PATH!())))
        .replace("# -continue_cases-", &continue_cases);

    let path = output_path.as_ref().join(engine.expand(PATH!()));
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
    for partition in lines.split_inclusive(|(_, _, command)| {
        matches!(*command, Line::Breakpoint | Line::FunctionCall { .. })
    }) {
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

            let path = function_directory.join("scheduled.mcfunction");
            let template = include_str!(
                "datapack_template/data/-ns-/functions/-orig_ns-/-orig/fn-/scheduled.mcfunction"
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
