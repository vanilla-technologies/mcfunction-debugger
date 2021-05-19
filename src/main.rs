mod parser;
mod utils;

use crate::parser::{parse_line, Line};
use clap::{App, Arg};
use const_format::concatcp;
use load_file::load_str;
use multimap::MultiMap;
use parser::commands::{EntityAnchor, NamespacedName};
use std::{
    collections::HashMap,
    fs::{create_dir_all, write},
    io::Error,
    path::{Path, PathBuf},
};
use std::{
    fs::File,
    io::{self, BufRead},
};
use walkdir::WalkDir;

const DATAPACK: &str = "datapack";
const OUTPUT: &str = "output";

const NAMESPACE: &str = "debug";

fn main() -> io::Result<()> {
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
        .get_matches();
    let datapack_path = Path::new(matches.value_of(DATAPACK).unwrap());
    let pack_mcmeta_path = datapack_path.join("pack.mcmeta");
    assert!(pack_mcmeta_path.is_file(), "Could not find pack.mcmeta");
    let output_path = Path::new(matches.value_of(OUTPUT).unwrap());
    generate_output_datapack(output_path)?;

    let functions = find_function_files(datapack_path)?;
    let output_function_path = output_path.join("data").join(NAMESPACE).join("functions");

    let function_contents = functions
        .iter()
        .map(|(name, path)| {
            let file = File::open(path)?;
            let lines = io::BufReader::new(file)
                .lines()
                .enumerate()
                .map(|(line_number, line)| {
                    line.map(|line| {
                        let command = parse_line(&line);
                        (line_number + 1, line, command)
                    })
                })
                .collect::<io::Result<Vec<(usize, String, Line)>>>()?;

            Ok((name, lines))
        })
        .collect::<Result<HashMap<&NamespacedName, Vec<(usize, String, Line)>>, io::Error>>()?;

    let call_tree = create_call_tree(&function_contents);

    for (name, lines) in function_contents.iter() {
        create_function_files(&output_function_path, name, lines, &call_tree)?;
    }

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

struct TemplateEngine<'l> {
    line_numbers: &'l str,
    original_namespace: &'l str,
    original_function: &'l str,
    namespace: &'l str,
}

impl TemplateEngine<'_> {
    fn expand(&self, template: &str) -> String {
        template
            .replace("original_namespace", self.original_namespace)
            .replace("original_function", self.original_function)
            .replace("line_numbers", self.line_numbers)
            .replace("namespace", self.namespace)
    }

    fn expand_line(&self, (line_number, line, command): &(usize, String, Line)) -> String {
        match command {
            Line::Breakpoint => {
                let template = include_str!("templates/set_breakpoint.mcfunction");
                let template = template.replace("line_number", &line_number.to_string());
                self.expand(&template)
            }
            Line::FunctionCall { name, anchor } => {
                let function_call = format!("function {}", name);
                let template = include_str!("templates/call_function.mcfunction");
                let execute = line.strip_suffix(&function_call).unwrap(); //TODO panic!
                let caller_function = self.original_function.replace("/", "_");
                let mut template = template
                    .replace("execute run ", execute)
                    .replace("callee_namespace", name.namespace())
                    .replace("callee_function", name.name())
                    .replace("caller_namespace", self.original_namespace)
                    .replace("caller_function", &caller_function);
                if let Some(EntityAnchor::EYES) = anchor {
                    let anchor_score =
                        format!("scoreboard players set current {}_anchor 1", NAMESPACE);
                    template = template.replace("# debug_anchor", &anchor_score);
                }
                self.expand(&template)
            }
            Line::OtherCommand => line.to_owned(),
        }
    }
}

fn create_function_files(
    output_function_path: &PathBuf,
    name: &NamespacedName,
    lines: &Vec<(usize, String, Line)>,
    call_tree: &MultiMap<&NamespacedName, (&NamespacedName, &usize)>,
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

        let engine = TemplateEngine {
            line_numbers: &line_numbers,
            original_namespace,
            original_function,
            namespace: NAMESPACE,
        };

        if first {
            let path = function_directory.join("iterate.mcfunction");
            let template  =include_str!(
                "templates/namespace/functions/original_namespace/original_function/iterate.mcfunction"
            );
            create_file(&path, &engine.expand(template))?;

            let path = function_directory.join("iteration_step.mcfunction");
            let template  =include_str!(
                "templates/namespace/functions/original_namespace/original_function/iteration_step.mcfunction"
            );
            create_file(&path, &engine.expand(template))?;

            let path = function_directory.join("start.mcfunction");
            let template  = include_str!(
                "templates/namespace/functions/original_namespace/original_function/start.mcfunction"
            );
            create_file(&path, &engine.expand(template))?;
        } else {
            let file_name = format!("{}_continue.mcfunction", start_line);
            let path = function_directory.join(file_name);
            let mut template = include_str!(
                "templates/namespace/functions/original_namespace/original_function/continue.mcfunction"
            ).to_string();
            if let Some(_callers) = call_tree.get_vec(name) {
                template.push_str(include_str!(
                    "templates/namespace/functions/original_namespace/original_function/continue_return.mcfunction"
                ));
            }
            create_file(&path, &engine.expand(&template))?;
        }
        start_line = end_line;

        // line_names.mcfunction
        let file_name = format!("{}.mcfunction", &line_numbers);
        let path = function_directory.join(file_name);
        let content = partition
            .iter()
            .map(|line| engine.expand_line(line))
            .collect::<Vec<_>>()
            .join("\n");
        let template  = include_str!(
                "templates/namespace/functions/original_namespace/original_function/line_numbers.mcfunction"
            );
        let template = template.replace("# content", &content);
        create_file(&path, &engine.expand(&template))?;

        // line_numbers_with_context.mcfunction
        let file_name = format!("{}_with_context.mcfunction", &line_numbers);
        let path = function_directory.join(file_name);
        let template  = include_str!(
            "templates/namespace/functions/original_namespace/original_function/line_numbers_with_context.mcfunction"
        );
        create_file(&path, &engine.expand(template))?;
    }

    if let Some(callers) = call_tree.get_vec(name) {
        let return_cases = callers
            .iter()
            .map(|(caller, line_number)| {
                format!(
                    "execute if entity @s[tag=namespace_{caller_namespace}_{caller_function_tag}] run \
                     function namespace:{caller_namespace}/{caller_function}/{line_number}_continue",
                    caller_namespace = caller.namespace(),
                    caller_function = caller.name(),
                    caller_function_tag = caller.name().replace("/", "_"),
                    line_number = *line_number + 1
                )
                .replace("original_function", original_function)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let content = include_str!(
            "templates/namespace/functions/original_namespace/original_function/return.mcfunction"
        )
        .replace("# return_cases", &return_cases)
        .replace("namespace", NAMESPACE);

        let path = function_directory.join("return.mcfunction");
        create_file(&path, &content)?;
    }

    Ok(())
}

fn generate_output_datapack(path: &Path) -> Result<(), io::Error> {
    const PREFIX: &str = "datapack_resources/";
    const ASSIGN: &str = "data/debug/functions/id/assign.mcfunction";
    create_file(path.join(ASSIGN), load_str!(concatcp!(PREFIX, ASSIGN)))?;
    const INIT: &str = "data/debug/functions/id/init_self.mcfunction";
    create_file(path.join(INIT), load_str!(concatcp!(PREFIX, INIT)))?;
    const INSTALL: &str = "data/debug/functions/id/install.mcfunction";
    create_file(path.join(INSTALL), load_str!(concatcp!(PREFIX, INSTALL)))?;
    const UNINSTALL: &str = "data/debug/functions/id/uninstall.mcfunction";
    create_file(
        path.join(UNINSTALL),
        load_str!(concatcp!(PREFIX, UNINSTALL)),
    )?;
    const SELECT_ENTITY: &str = "data/debug/functions/select_entity.mcfunction";
    create_file(
        path.join(SELECT_ENTITY),
        load_str!(concatcp!(PREFIX, SELECT_ENTITY)),
    )?;
    const PACK: &str = "pack.mcmeta";
    create_file(path.join(PACK), load_str!(concatcp!(PREFIX, PACK)))?;
    Ok(())
}

fn create_file<P: AsRef<Path>>(path: P, content: &str) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        create_dir_all(parent)?;
    }
    write(path, content)
}

fn find_function_files(
    datapack_path: &Path,
) -> Result<HashMap<NamespacedName, PathBuf>, io::Error> {
    let mut functions = HashMap::new();
    let data_path = datapack_path.join("data");
    if data_path.is_dir() {
        for entry in data_path.read_dir()? {
            let entry = entry?;
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
                                    functions.insert(name, path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(functions)
}
