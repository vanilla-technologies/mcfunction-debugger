mod parser;

use crate::parser::parseCommand;
use clap::{App, Arg};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use std::{
    fs::File,
    io::{self, BufRead},
};
use walkdir::WalkDir;

const DATAPACK: &str = "datapack";

fn main() -> io::Result<()> {
    let matches = App::new("mcfunction-debugger")
        .arg(
            Arg::with_name(DATAPACK)
                .long("datapack")
                .value_name("DIRECTORY")
                .takes_value(true)
                .required(true),
        )
        .get_matches();
    let datapack_path = Path::new(matches.value_of(DATAPACK).unwrap());
    let pack_mcmeta_path = datapack_path.join("pack.mcmeta");
    assert!(pack_mcmeta_path.is_file(), "Could not find pack.mcmeta");
    let functions = find_function_files(datapack_path)?;

    for (name, path) in functions.iter() {
        let file = File::open(path)?;
        for line in io::BufReader::new(file).lines() {
            let line = line?;
            let command = parseCommand(&line);
        }
    }

    println!("{:#?}", functions);
    Ok(())
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
