mod parser;
mod utils;

use crate::parser::{parse_line, Line};
use clap::{App, Arg};
use const_format::concatcp;
use load_file::load_str;
use std::{
    collections::HashMap,
    fs::{create_dir_all, write},
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

// templates
const LINE_NUMBERS: &str = include_str!(
    "templates/namespace/functions/original_namespace/original_function/line_numbers.mcfunction"
);

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
    generate_debug_datapack(output_path)?;

    let functions = find_function_files(datapack_path)?;
    let output_function_path = output_path.join("data").join(NAMESPACE).join("functions");
    for (name, path) in functions.iter() {
        let file = File::open(path)?;
        let lines = io::BufReader::new(file)
            .lines()
            .collect::<io::Result<Vec<_>>>()?;
        let function_directory = output_function_path.join(name.replace(":", "/"));
        create_dir_all(&function_directory)?;

        let mut start_line = 1;
        for part in lines.split_inclusive(|line| parse_line(line) != Line::OtherCommand) {
            let end_line = start_line + part.len();
            let file_name = format!("{}-{}.mcfunction", start_line, end_line - 1);
            let content = LINE_NUMBERS.replace("# content", &part.join("\n"));
            create_file(&function_directory.join(file_name), &content)?;
            start_line = end_line;
        }
    }

    // namespace_caller_namespace_caller_function
    // namespace
    // original_namespace
    // original_function
    // callee_namespace
    // callee_function
    // line_numbers
    // line_number
    // # scoreboard players set current namespace_anchor 1
    // # return_cases
    // # content
    // execute run

    Ok(())
}

fn generate_debug_datapack(path: &Path) -> Result<(), io::Error> {
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

fn find_function_files(datapack_path: &Path) -> Result<HashMap<String, PathBuf>, io::Error> {
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
                                    let function_name = format!(
                                        "{}:{}",
                                        namespace.to_string_lossy(),
                                        relative_path.with_extension("").display()
                                    );
                                    functions.insert(function_name, path);
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
