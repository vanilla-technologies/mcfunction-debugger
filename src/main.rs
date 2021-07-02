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
    fs::File,
    io::{self, BufRead},
    iter::{repeat, FromIterator},
    path::{Path, PathBuf},
};
use tokio::{
    fs::{create_dir_all, write},
    task::JoinHandle,
    try_join,
};
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

    generate_output_datapack(&function_contents, namespace, output_path).await?;

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

async fn generate_output_datapack<P: AsRef<Path>>(
    function_contents: &HashMap<&NamespacedName, Vec<(usize, String, Line)>>,
    namespace: &str,
    output_path: P,
) -> io::Result<()> {
    let engine = TemplateEngine::new(HashMap::from_iter([("-ns-", namespace)]));
    try_join!(
        expand_global_templates(&engine, function_contents, &output_path),
        expand_function_specific_templates(&engine, function_contents, &output_path),
    )?;
    Ok(())
}

macro_rules! expand_template {
    ($e:ident, $o:ident, $p:literal) => {{
        let path = $o.as_ref().join($e.expand($p));
        let content = $e.expand(include_str!(concat!("datapack_template/", $p)));
        write(path, content)
    }};
}

async fn expand_global_templates<P: AsRef<Path>>(
    engine: &TemplateEngine<'_>,
    function_contents: &HashMap<&NamespacedName, Vec<(usize, String, Line)>>,
    output_path: P,
) -> io::Result<()> {
    macro_rules! expand_template_local {
        ($p:literal) => {
            expand_template!(engine, output_path, $p)
        };
    }

    let output_path = output_path.as_ref();
    try_join!(
        create_dir_all(output_path.join(engine.expand("data/-ns-/functions/id")),),
        create_dir_all(output_path.join("data/minecraft/tags/functions")),
    )?;

    try_join!(
        expand_template_local!("data/-ns-/functions/id/assign.mcfunction"),
        expand_template_local!("data/-ns-/functions/id/init_self.mcfunction"),
        expand_template_local!("data/-ns-/functions/id/install.mcfunction"),
        expand_template_local!("data/-ns-/functions/id/uninstall.mcfunction"),
        expand_continue_aec_template(&engine, function_contents, &output_path,),
        expand_template_local!("data/-ns-/functions/continue.mcfunction"),
        expand_template_local!("data/-ns-/functions/decrement_age.mcfunction"),
        expand_template_local!("data/-ns-/functions/install.mcfunction"),
        expand_schedule_template(&engine, function_contents, &output_path,),
        expand_template_local!("data/-ns-/functions/select_entity.mcfunction"),
        expand_template_local!("data/-ns-/functions/tick_start.mcfunction"),
        expand_template_local!("data/-ns-/functions/tick.mcfunction"),
        expand_template_local!("data/-ns-/functions/uninstall.mcfunction"),
        expand_template_local!("data/minecraft/tags/functions/tick.json"),
        expand_template_local!("pack.mcmeta"),
    )?;

    Ok(())
}

async fn expand_continue_aec_template<P: AsRef<Path>>(
    engine: &TemplateEngine<'_>,
    function_contents: &HashMap<&NamespacedName, Vec<(usize, String, Line)>>,
    output_path: P,
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
    write(&path, &content).await
}

async fn expand_schedule_template<P: AsRef<Path>>(
    engine: &TemplateEngine<'_>,
    function_contents: &HashMap<&NamespacedName, Vec<(usize, String, Line)>>,
    output_path: P,
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
    write(&path, &content).await
}

async fn expand_function_specific_templates<P: AsRef<Path>>(
    engine: &TemplateEngine<'_>,
    function_contents: &HashMap<&NamespacedName, Vec<(usize, String, Line)>>,
    output_path: P,
) -> io::Result<()> {
    let call_tree = create_call_tree(&function_contents);

    try_join_all(function_contents.iter().map(|(fn_name, lines)| {
        expand_function_templates(&engine, fn_name, lines, &call_tree, &output_path)
    }))
    .await?;

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

async fn expand_function_templates<P: AsRef<Path>>(
    engine: &TemplateEngine<'_>,
    fn_name: &NamespacedName,
    lines: &Vec<(usize, String, Line)>,
    call_tree: &MultiMap<&NamespacedName, (&NamespacedName, &usize)>,
    output_path: P,
) -> io::Result<()> {
    let orig_fn = fn_name.name();
    let orig_fn_tag = orig_fn.replace('/', "_");
    let engine = engine.extend([
        ("-orig_ns-", fn_name.namespace()),
        ("-orig_fn-", &orig_fn_tag),
        ("-orig/fn-", orig_fn),
    ]);

    let fn_dir = output_path
        .as_ref()
        .join(engine.expand("data/-ns-/functions/-orig_ns-/-orig/fn-"));
    create_dir_all(&fn_dir).await?;

    let mut start_line = 1;
    for partition in lines.split_inclusive(|(_, _, command)| {
        matches!(*command, Line::Breakpoint | Line::FunctionCall { .. })
    }) {
        let first = start_line == 1;
        let end_line = start_line + partition.len();
        let line_numbers = format!("{}-{}", start_line, end_line - 1);

        let engine = engine.extend([("-line_numbers-", line_numbers.as_str())]);

        macro_rules! expand_template_local {
            ($p:literal) => {
                expand_template!(engine, output_path, $p)
            };
        }

        if first {
            try_join!(
                expand_template_local!(
                    "data/-ns-/functions/-orig_ns-/-orig/fn-/iterate.mcfunction"
                ),
                expand_template_local!(
                    "data/-ns-/functions/-orig_ns-/-orig/fn-/iteration_step.mcfunction"
                ),
                expand_template_local!(
                    "data/-ns-/functions/-orig_ns-/-orig/fn-/iterate_same_executor.mcfunction"
                ),
                expand_template_local!("data/-ns-/functions/-orig_ns-/-orig/fn-/start.mcfunction"),
                expand_template_local!(
                    "data/-ns-/functions/-orig_ns-/-orig/fn-/scheduled.mcfunction"
                ),
            )?;
        } else {
            let file_name = format!("{}_continue.mcfunction", start_line);
            let path = fn_dir.join(file_name);
            let mut template = include_str!(
                "datapack_template/data/-ns-/functions/-orig_ns-/-orig/fn-/continue.mcfunction"
            )
            .to_string();
            if let Some(_callers) = call_tree.get_vec(fn_name) {
                template.push_str(include_str!(
                    "datapack_template/data/-ns-/functions/-orig_ns-/-orig/fn-/continue_return.mcfunction"
                ));
            }
            write(&path, &engine.expand(&template)).await?;
        }
        start_line = end_line;

        // line_names.mcfunction
        let file_name = format!("{}.mcfunction", &line_numbers);
        let path = fn_dir.join(file_name);
        let content = partition
            .iter()
            .map(|line| engine.expand_line(line))
            .collect::<Vec<_>>()
            .join("\n");
        let template = include_str!(
            "datapack_template/data/-ns-/functions/-orig_ns-/-orig/fn-/-line_numbers-.mcfunction"
        );
        let template = template.replace("# -content-", &content);
        write(&path, &engine.expand(&template)).await?;

        // line_numbers_with_context.mcfunction
        let file_name = format!("{}_with_context.mcfunction", &line_numbers);
        let path = fn_dir.join(file_name);
        let template  = include_str!(
            "datapack_template/data/-ns-/functions/-orig_ns-/-orig/fn-/-line_numbers-_with_context.mcfunction"
        );
        write(&path, &engine.expand(template)).await?;
    }

    if let Some(callers) = call_tree.get_vec(fn_name) {
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

        let path = fn_dir.join("return.mcfunction");
        write(&path, &content).await?;
    }

    Ok(())
}
