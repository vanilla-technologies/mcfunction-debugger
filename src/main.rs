mod parser;
mod utils;

use crate::parser::parse_line;
use clap::{App, Arg};
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

// templates
const STORE_DEBUG_CALLER_MAIN_CONTEXT: &str =
    include_str!("templates/store_debug_caller_main_context.mcfunction");
const MARK_CURRENT_ENTITY: &str = include_str!("templates/mark_current_entity.mcfunction");

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
    for (name, path) in functions.iter() {
        let file = File::open(path)?;
        for line in io::BufReader::new(file).lines() {
            let line = line?;
            // let command = parse_line(&line);
        }
    }

    // function call => summon selected entity marker;
    //
    // set (directly before original function call))current;
    // call original function until beakpoint or file end

    // main context with execute parameters; call function that prepares original function call
    // restore context and call original function (until breakpoint)

    // init scoreboard, set debugger caller context, call program -> start
    // add depth, original code, set context, execute ... summon entity marker, call next function (leading to function in the original execute) -> main0
    // introduce recursive call -> set_stone
    // set current aec -> recursive_if_available / set_stone_if_available
    // restore entity (for current aec), call original function as entity -> set_stone0_aec
    // original function (until breakpoint) -> set_stone0

    // if not breakpoint, tidy up

    Ok(())
}

fn generate_debug_datapack(path: &Path) -> Result<(), io::Error> {
    create_file(
        path.join("data/debug/functions/id/assign.mcfunction"),
        include_str!("datapack_resources/debug/functions/id/assign.mcfunction"),
    )?;
    create_file(
        path.join("data/debug/functions/id/init_self.mcfunction"),
        include_str!("datapack_resources/debug/functions/id/init_self.mcfunction"),
    )?;
    create_file(
        path.join("data/debug/functions/id/install.mcfunction"),
        include_str!("datapack_resources/debug/functions/id/install.mcfunction"),
    )?;
    create_file(
        path.join("data/debug/functions/id/uninstall.mcfunction"),
        include_str!("datapack_resources/debug/functions/id/uninstall.mcfunction"),
    )?;
    create_file(
        path.join("data/debug/functions/summon_entity_markers/summon_selected_entity_marker.mcfunction"),
        include_str!("datapack_resources/debug/functions/summon_entity_markers/summon_selected_entity_marker.mcfunction"),
    )?;
    create_file(
        path.join("data/debug/functions/summon_entity_markers/summon_selected_entity_marker_anchored_eyes.mcfunction"),
        include_str!("datapack_resources/debug/functions/summon_entity_markers/summon_selected_entity_marker_anchored_eyes.mcfunction"),
    )?;
    create_file(
        path.join("pack.mcmeta"),
        include_str!("datapack_resources/pack.mcmeta"),
    )?;
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
