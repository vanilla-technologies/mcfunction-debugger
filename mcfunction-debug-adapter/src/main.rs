// mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any
// Minecraft mods.
//
// © Copyright (C) 2021, 2022 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
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

use clap::{crate_authors, crate_version, App};
use log::error;
use mcfunction_debug_adapter::{
    adapter::McfunctionDebugAdapter,
    codec::{ProtocolMessageDecoder, ProtocolMessageEncoder},
    error::DebugAdapterError,
    run_adapter,
};
use simplelog::{Config, WriteLogger};
use std::{io, path::Path};
use tokio_util::codec::{FramedRead, FramedWrite};

#[tokio::main]
async fn main() -> io::Result<()> {
    log_panics::init();

    App::new("mcfunction-debug-adapter")
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
© Copyright (C) 2021, 2022 {}

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
        .get_matches();

    let project_dir = Path::new(env!("PWD"));
    WriteLogger::init(
        log::LevelFilter::Trace,
        Config::default(),
        std::fs::File::create(project_dir.join("std.log"))?,
    )
    .unwrap();

    let input = FramedRead::new(tokio::io::stdin(), ProtocolMessageDecoder);
    let output = FramedWrite::new(tokio::io::stdout(), ProtocolMessageEncoder);
    match run_adapter(input, output, McfunctionDebugAdapter::new).await {
        Err(e) => {
            let e = match e {
                DebugAdapterError::Canceller(e) => e,
                DebugAdapterError::Output(e) => e,
                DebugAdapterError::Custom(e) => e,
            };
            error!("Stopping due to: {}", e);
            Err(e)
        }
        _ => Ok(()),
    }
}
