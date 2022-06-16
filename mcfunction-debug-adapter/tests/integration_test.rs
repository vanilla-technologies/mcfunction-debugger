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

use assert2::{check, let_assert};
use debug_adapter_protocol::{
    events::Event,
    requests::{InitializeRequestArguments, LaunchRequestArguments},
    responses::{ErrorResponse, Response, SuccessResponse},
    ProtocolMessage, ProtocolMessageContent as Content, SequenceNumber,
};
use futures::{Sink, SinkExt, Stream, StreamExt};
use mcfunction_debug_adapter::adapter::McfunctionDebugAdapter;
use mcfunction_debugger::parser::command::resource_location::ResourceLocation;
use minect::{log_observer::LogObserver, LoggedCommand};
use sender_sink::wrappers::UnboundedSenderSink;
use serde_json::{json, Map};
use std::{
    fs::{create_dir_all, write},
    io,
    iter::FromIterator,
    path::{Path, PathBuf},
};
use tokio::task::JoinHandle;
use tokio_stream::wrappers::UnboundedReceiverStream;

const ADAPTER_ID: &str = "mcfunction";
const LISTENER_NAME: &str = "test";
const TEST_LOG_FILE: &str = env!("TEST_LOG_FILE");
const TEST_WORLD_DIR: &str = env!("TEST_WORLD_DIR");

#[tokio::test]
async fn test() -> io::Result<()> {
    let test = Mcfunction {
        name: ResourceLocation::new("test", "bla"),
        lines: vec![
            logged_command("function minect:enable_logging"),
            named_logged_command("tag @s add some_tag"),
            logged_command("function minect:reset_logging"),
        ],
    };
    let test_fn_path = test.full_path().display().to_string();
    create_datapack(vec![test]);

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut log_listener = log_observer.add_listener(LISTENER_NAME);

    let (handle, adapter_input, mut adapter_output) = start_adapter();
    let mut adapter_input = ProtocolMessageSender::new(adapter_input);

    let content = InitializeRequestArguments::builder()
        .adapter_id(ADAPTER_ID.to_string())
        .build();
    let request_seq = adapter_input.send_ok(content).await;

    let event = adapter_output.next().await.unwrap();
    assert_eq!(event.content, Content::Event(Event::Initialized));

    let response = adapter_output.next().await.unwrap();
    check!(let SuccessResponse::Initialize(_) = assert_success_response(response, request_seq));

    let content = LaunchRequestArguments::builder()
        .additional_attributes(Map::from_iter([
            ("minecraftLogFile".to_string(), json!(TEST_LOG_FILE)),
            ("minecraftWorldDir".to_string(), json!(TEST_WORLD_DIR)),
            ("program".to_string(), json!(test_fn_path)),
        ]))
        .build();
    let request_seq = adapter_input.send_ok(content).await;

    let response = adapter_output.next().await.unwrap();
    check!(let SuccessResponse::Launch = assert_success_response(response, request_seq));

    let event = adapter_output.next().await.unwrap();
    check!(let Content::Event(Event::Terminated(_)) = event.content);

    let log_event = log_listener.recv().await.unwrap();
    check!(log_event.message == "Added tag 'some_tag' to test");

    handle.await.unwrap().unwrap();
    Ok(())
}

#[tokio::test]
async fn test_program_not_in_data_directory_of_datapack() -> io::Result<()> {
    create_datapack(Vec::new());
    let test_fn_path = datapack_dir().join("not-data").join("test.mcfunction");

    let (_handle, adapter_input, mut adapter_output) = start_adapter();
    let mut adapter_input = ProtocolMessageSender::new(adapter_input);

    let content = InitializeRequestArguments::builder()
        .adapter_id(ADAPTER_ID.to_string())
        .build();
    let request_seq = adapter_input.send_ok(content).await;

    let event = adapter_output.next().await.unwrap();
    assert_eq!(event.content, Content::Event(Event::Initialized));

    let response = adapter_output.next().await.unwrap();
    check!(let SuccessResponse::Initialize(_) = assert_success_response(response, request_seq));

    let content = LaunchRequestArguments::builder()
        .additional_attributes(Map::from_iter([
            ("minecraftLogFile".to_string(), json!(TEST_LOG_FILE)),
            ("minecraftWorldDir".to_string(), json!(TEST_WORLD_DIR)),
            ("program".to_string(), json!(test_fn_path)),
        ]))
        .build();
    let request_seq = adapter_input.send_ok(content).await;

    let response = adapter_output.next().await.unwrap();

    let error_response = assert_error_response(response, request_seq);
    check!(error_response.command == "launch");
    check!(error_response
        .message
        .starts_with("Attribute 'program' does not denote a path in the data directory"));
    Ok(())
}

fn assert_success_response(
    response: ProtocolMessage,
    expected_request_seq: SequenceNumber,
) -> SuccessResponse {
    let_assert!(
        Content::Response(Response {
            request_seq,
            result: Ok(success_response)
        }) = response.content
    );
    assert_eq!(request_seq, expected_request_seq);
    success_response
}

fn assert_error_response(
    response: ProtocolMessage,
    expected_request_seq: SequenceNumber,
) -> ErrorResponse {
    let_assert!(
        Content::Response(Response {
            request_seq,
            result: Err(error_response)
        }) = response.content
    );
    assert_eq!(request_seq, expected_request_seq);
    error_response
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

    async fn send_ok(&mut self, content: impl Into<Content>) -> SequenceNumber {
        self.seq += 1;
        let msg = ProtocolMessage::new(self.seq, content);
        self.send(Ok(msg)).await;
        self.seq
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

fn named_logged_command(command: &str) -> String {
    LoggedCommand::builder(command.to_string())
        .name(LISTENER_NAME)
        .build()
        .to_string()
}

fn logged_command(command: &str) -> String {
    LoggedCommand::builder(command.to_string())
        .build()
        .to_string()
}

struct Mcfunction {
    name: ResourceLocation,
    lines: Vec<String>,
}
impl Mcfunction {
    fn full_path(&self) -> PathBuf {
        datapack_dir()
            .join("data")
            .join(self.name.namespace())
            .join("functions")
            .join(self.name.path())
            .with_extension("mcfunction")
    }
}

const TEST_DATAPACK_NAME: &str = "adapter-test";

fn create_datapack(functions: Vec<Mcfunction>) {
    create_dir_all(&datapack_dir()).unwrap();
    write(
        datapack_dir().join("pack.mcmeta"),
        r#"{"pack":{"pack_format":7,"description":"mcfunction-debugger test tick"}}"#,
    )
    .unwrap();
    for function in functions {
        let path = function.full_path();
        create_dir_all(&path.parent().unwrap()).unwrap();
        write(path, function.lines.join("\n")).unwrap();
    }
}

fn datapack_dir() -> std::path::PathBuf {
    Path::new(TEST_WORLD_DIR)
        .join("datapacks")
        .join(TEST_DATAPACK_NAME)
}
