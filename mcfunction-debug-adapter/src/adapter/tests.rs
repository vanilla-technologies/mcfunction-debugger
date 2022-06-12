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
use assert2::{check, let_assert};
use debug_adapter_protocol::{
    events::Event,
    requests::InitializeRequestArguments,
    responses::{Response, SuccessResponse},
    ProtocolMessage, ProtocolMessageType, SequenceNumber,
};
use futures::{Sink, SinkExt, Stream, StreamExt};
use mcfunction_debugger::parser::command::resource_location::ResourceLocation;
use minect::LoggedCommand;
use sender_sink::wrappers::UnboundedSenderSink;
use std::{
    fs::{create_dir_all, write},
    io,
    path::Path,
};
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

    let (handle, adapter_input, mut adapter_output) = start_adapter();
    let mut adapter_input = ProtocolMessageSender::new(adapter_input);

    let payload = InitializeRequestArguments::builder()
        .adapter_id("adapter_id".to_string())
        .build();
    let request_seq = adapter_input.send_ok(payload).await;

    let event = adapter_output.next().await.unwrap();
    assert_eq!(event.type_, ProtocolMessageType::Event(Event::Initialized));

    let response = adapter_output.next().await.unwrap();
    check!(let SuccessResponse::Initialize(_) = assert_success_response(response, request_seq));

    // handle.await.unwrap().unwrap();
    Ok(())
}

fn assert_success_response(
    response: ProtocolMessage,
    expected_request_seq: SequenceNumber,
) -> SuccessResponse {
    let_assert!(
        ProtocolMessageType::Response(Response {
            request_seq,
            result: Ok(success_response)
        }) = response.type_
    );
    assert_eq!(request_seq, expected_request_seq);
    success_response
}

struct ProtocolMessageSender<I>
where
    I: Sink<io::Result<ProtocolMessage>, Error = io::Error>,
{
    seq: SequenceNumber,
    adapter_input: I,
}
impl<I> ProtocolMessageSender<I>
where
    I: Sink<io::Result<ProtocolMessage>, Error = io::Error> + Unpin,
{
    fn new(adapter_input: I) -> ProtocolMessageSender<I> {
        ProtocolMessageSender {
            seq: 0,
            adapter_input,
        }
    }

    async fn send_ok(&mut self, payload: impl Into<ProtocolMessageType>) -> SequenceNumber {
        self.seq += 1;
        let msg = ProtocolMessage::new(self.seq, payload);
        self.send(Ok(msg)).await;
        self.seq
    }

    async fn send_err(&mut self, payload: impl Into<io::Error>) {
        self.send(Err(payload.into())).await;
    }

    async fn send(&mut self, msg: impl Into<io::Result<ProtocolMessage>>) {
        self.adapter_input.send(msg.into()).await.unwrap();
    }
}

fn start_adapter() -> (
    JoinHandle<io::Result<()>>,
    impl Sink<io::Result<ProtocolMessage>, Error = io::Error>,
    impl Stream<Item = ProtocolMessage>,
) {
    let (adapter_input_sink, adapter_input_stream) = unbound_io_channel();
    let (adapter_output_sink, adapter_output_stream) = unbound_io_channel();
    let mut adapter = McfunctionDebugAdapter::new(adapter_input_stream, adapter_output_sink);
    let handle = tokio::task::spawn(async move { adapter.run().await });
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

struct Mcfunction {
    name: ResourceLocation,
    lines: Vec<String>,
}

const TEST_WORLD_DIR: &str = env!("TEST_WORLD_DIR");

fn create_datapack(functions: Vec<Mcfunction>) {
    let dir = Path::new(TEST_WORLD_DIR).join("datapacks/adapter-test");
    create_dir_all(&dir).unwrap();
    write(
        dir.join("pack.mcmeta"),
        r#"{"pack":{"pack_format":7,"description":"mcfunction-debugger test tick"}}"#,
    )
    .unwrap();
    for function in functions {
        let path = dir
            .join(function.name.namespace())
            .join("data")
            .join(function.name.path())
            .with_extension("mcfunction");
        create_dir_all(&path.parent().unwrap()).unwrap();
        write(path, function.lines.join("\n")).unwrap();
    }
}
