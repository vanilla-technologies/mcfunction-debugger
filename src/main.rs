// mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any
// Minecraft mods.
//
// © Copyright (C) 2021 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
//
// This file is part of mcfunction-debugger.
//
// mcfunction-debugger is free software: you can redistribute it and/or modify it under the terms of
// the GNU General Public License as published by the Free Software Foundation, either version 3 of
// the License, or (at your option) any later version.
//
// mcfunction-debugger is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with mcfunction-debugger.
// If not, see <http://www.gnu.org/licenses/>.

use clap::{crate_authors, crate_version, App, Arg};
use log::LevelFilter;
use mcfunction_debugger::{generate_debug_datapack, Config};
use simple_logger::SimpleLogger;
use std::{io, path::Path};

const INPUT_ARG: &str = "datapack";
const OUTPUT_ARG: &str = "output";
const NAMESPACE_ARG: &str = "namespace";
const SHADOW_ARG: &str = "shadow";
const LOG_LEVEL_ARG: &str = "log-level";

// Copy of private field log::LOG_LEVEL_NAMES
const LOG_LEVEL_NAMES: [&str; 6] = ["OFF", "ERROR", "WARN", "INFO", "DEBUG", "TRACE"];
const LOG_LEVELS: [LevelFilter; 6] = [
    LevelFilter::Off,
    LevelFilter::Error,
    LevelFilter::Warn,
    LevelFilter::Info,
    LevelFilter::Debug,
    LevelFilter::Trace,
];

#[tokio::main]
async fn main() -> io::Result<()> {
    let matches = App::new("mcfunction-debugger")
        .version(crate_version!())
        .long_version(concat!(
            crate_version!(),
            " (Commit: ",
            env!("VERGEN_GIT_SHA"),
            ")"
        ))
        .version_short("v")
        .author(&*format!(
            "
Vanilla Technologies
© Copyright (C) 2021 {}

mcfunction-debugger is free software: you can redistribute it and/or modify it
under the terms of the GNU General Public License as published by the Free
Software Foundation, either version 3 of the License, or (at your option) any
later version.
mcfunction-debugger is distributed in the hope that it will be useful, but
WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
See the GNU General Public License for more details.

",
            crate_authors!(" & ")
        ))
        .about("Generate debug datapacks that suspend on '# breakpoint' lines")
        .long_about(
            "Debug your datapacks in five steps:\n\
            1. Add '# breakpoint' lines in your *.mcfunction files\n\
            2. Generate a debug datapack\n\
            3. Load the debug datapack in Minecraft\n\
            4. Start debugging any of your functions with: \
            /function debug:<your_namespace>/<your_function>\n\
            5. When finished, uninstall the debug datapack with: \
            /function debug:uninstall",
        )
        .arg(
            Arg::with_name(INPUT_ARG)
                .help("The input datapack directory.")
                .long_help(
                    "The datapack to generate a debug datapack for. Must be a directory containing \
                    a pack.mcmeta file, for example: \
                    '%APPDATA%\\.minecraft\\saves\\Your-World\\datapacks\\my-datapack'.",
                )
                .long("input")
                .value_name("DATAPACK")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(OUTPUT_ARG)
                .help("The output datapack directory.")
                .long_help(
                    "The directory that should become the generated debug datapack. On Windows \
                    this is typically a directory in the datapacks directory of your world, for \
                    example: \
                    '%APPDATA%\\.minecraft\\saves\\Your-World\\datapacks\\debug-my-datapack'.",
                )
                .long("output")
                .value_name("DATAPACK")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(NAMESPACE_ARG)
                .help("The internal namespace of the generated datapack.")
                .long_help(
                    "The namespace is used for all internal functions in the generated datapack \
                    and as a prefix for all scoreboard objectives and tags. By specifying a \
                    different namespace with max. 7 characters you can avoid name clashes. The \
                    generated functions in the 'debug' namespace such as 'debug:resume' and \
                    'debug:uninstall' are unaffected by this option.",
                )
                .long("namespace")
                .value_name("STRING")
                .takes_value(true)
                .default_value("mcfd")
                .validator(|namespace| {
                    if namespace.len() <= 7 {
                        // max len of identifiers 16 => scoreboard {}_Duration has 9 characters -> 7 remaining for namespace
                        return Ok(());
                    }
                    Err(String::from("string must have <= 7 characters"))
                }),
        )
        .arg(
            Arg::with_name(SHADOW_ARG)
                .help(
                    "Whether to generate debug functions with the same name as the original \
            functions.",
                )
                .long_help(
                    "When this is true the generated datapack will additionally contain functions \
                    with the same name as the functions in the input datapack. These functions \
                    will simply forward to the appropriate function in the 'debug' namespace. When \
                    using this make sure to disable the input datapack to avoid name clashes.\n\n\
                    This can be helpful when executing a function from a command block, because \
                    you don't have to change the function call to debug the function. Note however \
                    that calling a debug function inside an execute command prevents the debugger \
                    from suspending the execute command. For example, if the command \
                    'execute as @e run function my_namespace:my_function' hits a breakpoint in \
                    my_function and there is more than one entity, my_function will be called \
                    again, resulting in an error like: \
                    'Cannot start debugging my_namespace:my_function, because a function is \
                    already suspended at a breakpoint!'",
                )
                .long("shadow"),
        )
        .arg(
            Arg::with_name(LOG_LEVEL_ARG)
                .long_help(
                    "The log level can also be configured via the environment variable \
                    'LOG_LEVEL'.",
                )
                .long("log-level")
                .value_name("LOG_LEVEL")
                .takes_value(true)
                .env("LOG_LEVEL")
                .possible_values(&LOG_LEVEL_NAMES)
                .default_value(LevelFilter::Info.as_str()),
        )
        .get_matches();
    let input_path = Path::new(matches.value_of(INPUT_ARG).unwrap());
    let output_path = Path::new(matches.value_of(OUTPUT_ARG).unwrap());
    let namespace = matches.value_of(NAMESPACE_ARG).unwrap();
    let shadow = matches.is_present(SHADOW_ARG);
    let log_level = parse_log_level(matches.value_of(LOG_LEVEL_ARG).unwrap()).unwrap();

    SimpleLogger::new().with_level(log_level).init().unwrap();

    let pack_mcmeta_path = input_path.join("pack.mcmeta");
    assert!(pack_mcmeta_path.is_file(), "Could not find pack.mcmeta");

    let config = Config {
        namespace,
        shadow,
        adapter: None,
    };
    generate_debug_datapack(input_path, output_path, &config).await?;

    Ok(())
}

fn parse_log_level(log_level: &str) -> Option<LevelFilter> {
    let index = LOG_LEVEL_NAMES.iter().position(|&it| it == log_level)?;
    Some(LOG_LEVELS[index])
}
