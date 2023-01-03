// mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any
// Minecraft mods.
//
// © Copyright (C) 2021-2023 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
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
use log::{error, LevelFilter};
use mcfunction_debug_adapter::{
    adapter::McfunctionDebugAdapter,
    codec::{ProtocolMessageDecoder, ProtocolMessageEncoder},
    run_adapter,
};
use simplelog::{Config, WriteLogger};
use std::io::{self};
use tokio_util::codec::{FramedRead, FramedWrite};

const LOG_FILE_ARG: &str = "log-file";
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
    log_panics::init();

    let matches = App::new("mcfunction-debug-adapter")
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
© Copyright (C) 2021-2023 {}

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
        .arg(
            Arg::with_name(LOG_FILE_ARG)
                .help("Path at which to create a log file.")
                .long("log-file")
                .value_name("LOG_FILE")
                .takes_value(true),
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

    if let Some(log_file) = matches.value_of(LOG_FILE_ARG) {
        let log_level = parse_log_level(matches.value_of(LOG_LEVEL_ARG).unwrap()).unwrap();
        let log_file = std::fs::File::create(log_file)?;
        WriteLogger::init(log_level, Config::default(), log_file).unwrap();
    }

    let input = FramedRead::new(tokio::io::stdin(), ProtocolMessageDecoder);
    let output = FramedWrite::new(tokio::io::stdout(), ProtocolMessageEncoder);
    run_adapter(input, output, McfunctionDebugAdapter::new)
        .await
        .map_err(|e| {
            let e = e.into_inner();
            error!("Stopping due to: {}", e);
            e
        })
}

fn parse_log_level(log_level: &str) -> Option<LevelFilter> {
    let index = LOG_LEVEL_NAMES.iter().position(|&it| it == log_level)?;
    Some(LOG_LEVELS[index])
}
