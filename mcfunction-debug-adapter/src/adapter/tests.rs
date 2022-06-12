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

use crate::adapter::McfunctionDebugAdapter;
use debug_adapter_protocol::{
    requests::{InitializeRequestArguments, Request},
    ProtocolMessage, ProtocolMessageType,
};
use futures::{Sink, SinkExt, Stream, StreamExt};
use mcfunction_debugger::parser::command::resource_location::ResourceLocation;
use minect::LoggedCommand;
use sender_sink::wrappers::UnboundedSenderSink;
use std::io;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::UnboundedReceiverStream;

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

    let (handle, mut adapter_input, mut adapter_output) = start_adapter();

    let a = adapter_output.next().await;
    handle.await??;

    adapter_input
        .send(Ok(ProtocolMessage::new(
            1,
            InitializeRequestArguments::builder()
                .adapter_id("adapter_id".to_string())
                .build(),
        )))
        .await
        .unwrap();

    Ok(())
}

fn start_adapter() -> (
    JoinHandle<io::Result<()>>,
    impl Sink<io::Result<ProtocolMessage>, Error = io::Error>,
    impl Stream<Item = ProtocolMessage>,
) {
    let (adapter_input_sink, adapter_input_stream) = unbound_io_channel();
    let (adapter_output_sink, adapter_output_stream) = unbound_io_channel();
    let mut adapter = McfunctionDebugAdapter::new(adapter_input_stream, adapter_output_sink);
    let handle = tokio::task::spawn_local(async move { adapter.run().await });
    (handle, adapter_input_sink, adapter_output_stream)
}

fn unbound_io_channel<I>() -> (impl Sink<I, Error = io::Error>, impl Stream<Item = I>) {
    let (send, recv) = tokio::sync::mpsc::unbounded_channel();
    let sink = UnboundedSenderSink::from(send)
        .sink_map_err(|_| io::Error::new(io::ErrorKind::ConnectionAborted, ""));
    let stream = UnboundedReceiverStream::new(recv);
    (sink, stream)
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
