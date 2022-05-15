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

use std::io;

use bytes::Bytes;
use debug_adapter_protocol::{
    requests::{InitializeRequestArguments, Request},
    ProtocolMessage, ProtocolMessageType,
};
use mcfunction_debugger::parser::command::resource_location::ResourceLocation;
use minect::LoggedCommand;
use tokio::{fs::File, sync::mpsc::UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::io::StreamReader;

use super::McfunctionDebugAdapter;

const LISTENER_NAME: &str = "test";

#[tokio::test]
async fn bla() -> io::Result<()> {
    let test = Mcfunction {
        name: ResourceLocation::new("test", "bla"),
        lines: vec![
            logged_command("tag @s add line1"),
            logged_command("tag @s add line2"),
        ],
    };

    create_datapack(vec![test]);

    let (send, recv): (UnboundedSender<io::Result<Bytes>>, _) =
        tokio::sync::mpsc::unbounded_channel();

    let read = StreamReader::new(UnboundedReceiverStream::new(recv));

    let f = File::create("bla").await?;
    let mut under_test = McfunctionDebugAdapter::new(read, f);
    let handle = tokio::task::spawn_local(async move { under_test.run().await });

    send.send(Ok(Bytes::from(
        ProtocolMessage {
            seq: 1,
            type_: ProtocolMessageType::Request(Request::Initialize(InitializeRequestArguments {
                client_id: todo!(),
                client_name: todo!(),
                adapter_id: todo!(),
                locale: todo!(),
                lines_start_at_1: todo!(),
                columns_start_at_1: todo!(),
                path_format: todo!(),
                supports_variable_type: todo!(),
                supports_variable_paging: todo!(),
                supports_run_in_terminal_request: todo!(),
                supports_memory_references: todo!(),
                supports_progress_reporting: todo!(),
                supports_invalidated_event: todo!(),
            })),
        }
        .to_string()
        .as_bytes(),
    )))
    .unwrap();

    handle.await??;
    Ok(())
}

fn logged_command(command: &str) -> String {
    LoggedCommand::builder(command.to_string())
        .name(LISTENER_NAME)
        .build()
        .to_string()
}

fn create_datapack(lines: Vec<Mcfunction>) {
    todo!()
}

struct Mcfunction {
    name: ResourceLocation,
    lines: Vec<String>,
}

fn create(function: Mcfunction) {
    todo!()
}
