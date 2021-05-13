mod parser;

use crate::parser::parse_command;
use clap::{App, Arg};
use std::{
    collections::HashMap,
    fs::{create_dir, write},
    path::{Path, PathBuf},
};
use std::{
    fs::File,
    io::{self, BufRead},
};
use walkdir::WalkDir;

const DATAPACK: &str = "datapack";
const OUTPUT: &str = "output";

// datapack resources
static PACK_MCMETA: &'static str = include_str!("datapack_resources/pack.mcmeta");
static ID_INSTALL: &'static str =
    include_str!("datapack_resources/id_generation/install.mcfunction");
static ID_UNINSTALL: &'static str =
    include_str!("datapack_resources/id_generation/uninstall.mcfunction");
static ID_INIT: &'static str =
    include_str!("datapack_resources/id_generation/init_self.mcfunction");
static ID_ASSIGN: &'static str = include_str!("datapack_resources/id_generation/assign.mcfunction");

// templates
static STORE_DEBUG_CALLER_MAIN_CONTEXT: &'static str =
    include_str!("templates/store_debug_caller_main_context.mcfunction");
static MARK_CURRENT_ENTITY: &'static str = include_str!("templates/mark_current_entity.mcfunction");

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
    let debug_data_output_path = generate_debug_datapack(output_path);

    let functions = find_function_files(datapack_path)?;
    for (name, path) in functions.iter() {
        let file = File::open(path)?;
        for line in io::BufReader::new(file).lines() {
            let line = line?;
            let command = parse_command(&line);
        }
    }

    // function call => summon selected entity marker;
    //
    // set (directly before original function call))current;
    // call original function until beakpoint or file end

    // main context with execute parameters; call function that prepares original function call
    // restore context and call original function (until breakpoint)

    Ok(())
}

fn generate_debug_datapack(datapack_path: &Path) -> Result<PathBuf, io::Error> {
    let mut datapack_path = datapack_path.join("debug");
    create_dir(&datapack_path)?;
    let mcmeta_file = datapack_path.join("pack.mcmeta");
    write(mcmeta_file, PACK_MCMETA)?;
    let data_directory = datapack_path.join("data");
    create_dir(&data_directory)?;

    // add id generation mcfunction files
    let id_generation_directory = data_directory.join("id");
    create_dir(&id_generation_directory)?;
    write(
        id_generation_directory.join("install.mcfunction"),
        ID_INSTALL,
    )?;
    write(
        id_generation_directory.join("init_self.mcfunction"),
        ID_INIT,
    )?;
    write(id_generation_directory.join("assign.mcfunction"), ID_ASSIGN)?;
    write(
        id_generation_directory.join("uninstall.mcfunction"),
        ID_UNINSTALL,
    )?;

    return Ok(data_directory);
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
