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

pub mod timeout;

use crate::utils::timeout::TimeoutStream;
use assert2::{assert, let_assert};
use debug_adapter_protocol::{
    events::{Event, StoppedEventReason},
    requests::{
        ContinueRequestArguments, InitializeRequestArguments, LaunchRequestArguments,
        SetBreakpointsRequestArguments,
    },
    responses::{ErrorResponse, Response, SetBreakpointsResponseBody, SuccessResponse},
    types::{Source, SourceBreakpoint},
    ProtocolMessage, ProtocolMessageContent as Content, SequenceNumber,
};
use futures::{Sink, SinkExt, Stream};
use mcfunction_debug_adapter::adapter::McfunctionDebugAdapter;
use mcfunction_debugger::parser::command::resource_location::ResourceLocation;
use minect::{LoggedCommand, MinecraftConnection};
use sender_sink::wrappers::UnboundedSenderSink;
use serde_json::{json, Map};
use std::{
    fs::{create_dir_all, write},
    io,
    iter::FromIterator,
    path::{Path, PathBuf},
};
use timeout::DEFAULT_TIMEOUT;
use tokio::{task::JoinHandle, time::timeout};
use tokio_stream::wrappers::UnboundedReceiverStream;

const ADAPTER_ID: &str = "mcfunction";
pub const TEST_DATAPACK_NAME: &str = "adapter-test";
pub const LISTENER_NAME: &str = "adapter-test-listener";
pub const TEST_LOG_FILE: &str = env!("TEST_LOG_FILE");
pub const TEST_WORLD_DIR: &str = env!("TEST_WORLD_DIR");
const THREAD_ID: i32 = 0;

pub struct TestAdapter<I, O>
where
    I: Sink<io::Result<ProtocolMessage>, Error = io::Error> + Unpin,
    O: Stream<Item = ProtocolMessage> + Unpin,
{
    pub handle: JoinHandle<io::Result<()>>,
    pub input: ProtocolMessageSender<I>,
    pub output: TimeoutStream<O, ProtocolMessage>,
}

pub fn start_adapter() -> TestAdapter<
    impl Sink<io::Result<ProtocolMessage>, Error = io::Error> + Unpin,
    impl Stream<Item = ProtocolMessage> + Unpin,
> {
    let (input, adapter_input_stream) = unbound_io_channel();
    let (adapter_output_sink, output) = unbound_io_channel();
    let mut adapter = McfunctionDebugAdapter::new(adapter_input_stream, adapter_output_sink);
    let handle = tokio::task::spawn(async move { adapter.run().await });

    let adapter_input: Box<dyn Sink<io::Result<ProtocolMessage>, Error = io::Error> + Unpin> =
        Box::new(input);
    let input = ProtocolMessageSender::new(adapter_input);

    TestAdapter {
        handle,
        input,
        output: TimeoutStream::new(output),
    }
}

impl<I, O> TestAdapter<I, O>
where
    I: Sink<io::Result<ProtocolMessage>, Error = io::Error> + Unpin,
    O: Stream<Item = ProtocolMessage> + Unpin,
{
    pub async fn assert_stopped_at_breakpoint(&mut self) {
        let event = self.output.next().await.unwrap();
        let_assert!(Content::Event(Event::Stopped(body)) = event.content);
        assert!(body.reason == StoppedEventReason::Breakpoint);
    }

    pub async fn assert_terminated(mut self) {
        let event = self.output.next().await.unwrap();
        assert!(let Content::Event(Event::Terminated(_)) = event.content);

        timeout(DEFAULT_TIMEOUT, self.handle)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }

    pub async fn continue_(&mut self) {
        let content = ContinueRequestArguments::builder()
            .thread_id(THREAD_ID)
            .build();
        let request_seq = self.input.send_ok(content).await;

        let response = self.output.next().await.unwrap();
        assert!(let SuccessResponse::Continue(_) = assert_success_response(response, request_seq));
    }

    pub async fn initalize(&mut self) {
        let content = InitializeRequestArguments::builder()
            .adapter_id(ADAPTER_ID.to_string())
            .build();
        let request_seq = self.input.send_ok(content).await;

        let event = self.output.next().await.unwrap();
        assert_eq!(event.content, Content::Event(Event::Initialized));

        let response = self.output.next().await.unwrap();
        assert!(let SuccessResponse::Initialize(_) = assert_success_response(response, request_seq));
    }

    pub async fn launch(&mut self, test_fn_path: impl AsRef<Path>) {
        let request_seq = self.send_launch(test_fn_path).await;

        let response = self.output.next().await.unwrap();
        assert!(let SuccessResponse::Launch = assert_success_response(response, request_seq));
    }
    pub async fn send_launch(&mut self, test_fn_path: impl AsRef<Path>) -> u64 {
        let test_fn_path = test_fn_path.as_ref().display().to_string();
        let content = LaunchRequestArguments::builder()
            .additional_attributes(Map::from_iter([
                ("minecraftLogFile".to_string(), json!(TEST_LOG_FILE)),
                ("minecraftWorldDir".to_string(), json!(TEST_WORLD_DIR)),
                ("program".to_string(), json!(test_fn_path)),
            ]))
            .build();
        self.input.send_ok(content).await
    }

    pub async fn set_breakpoints_verified(
        &mut self,
        path: impl AsRef<Path>,
        breakpoints: &[SourceBreakpoint],
    ) {
        let response = self.set_breakpoints(path, breakpoints).await;
        assert_all_breakpoints_verified(&response, breakpoints);
    }
    pub async fn set_breakpoints(
        &mut self,
        path: impl AsRef<Path>,
        breakpoints: &[SourceBreakpoint],
    ) -> SetBreakpointsResponseBody {
        self.set_breakpoints_source_modified(path, breakpoints, false)
            .await
    }
    pub async fn set_breakpoints_source_modified(
        &mut self,
        path: impl AsRef<Path>,
        breakpoints: &[SourceBreakpoint],
        source_modified: bool,
    ) -> SetBreakpointsResponseBody {
        let content = SetBreakpointsRequestArguments::builder()
            .source(
                Source::builder()
                    .path(Some(path.as_ref().display().to_string()))
                    .build(),
            )
            .breakpoints(breakpoints.into())
            .source_modified(source_modified)
            .build();
        let request_seq = self.input.send_ok(content).await;
        let response = self.output.next().await.unwrap();
        let_assert!(
            SuccessResponse::SetBreakpoints(body) = assert_success_response(response, request_seq)
        );
        body
    }
}
pub fn assert_all_breakpoints_verified(
    response: &SetBreakpointsResponseBody,
    breakpoints: &[SourceBreakpoint],
) {
    assert!(response.breakpoints.len() == breakpoints.len());
    for (breakpoint, source_breakpoint) in response.breakpoints.iter().zip(breakpoints) {
        assert!(breakpoint.line == Some(source_breakpoint.line));
        assert!(breakpoint.verified == true);
    }
}

fn unbound_io_channel<I>() -> (impl Sink<I, Error = io::Error>, impl Stream<Item = I>) {
    let (send, recv) = tokio::sync::mpsc::unbounded_channel();
    let sink = UnboundedSenderSink::from(send)
        .sink_map_err(|_| io::Error::new(io::ErrorKind::ConnectionAborted, ""));
    let stream = UnboundedReceiverStream::new(recv);
    (sink, stream)
}

pub struct ProtocolMessageSender<I>
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

    pub async fn send_ok(&mut self, content: impl Into<Content>) -> SequenceNumber {
        self.seq += 1;
        let msg = ProtocolMessage::new(self.seq, content);
        self.send(Ok(msg)).await;
        self.seq
    }

    pub async fn send(&mut self, msg: impl Into<io::Result<ProtocolMessage>>) {
        self.adapter_input.send(msg.into()).await.unwrap();
    }
}

#[derive(Clone)]
pub struct Mcfunction {
    pub name: ResourceLocation,
    pub lines: Vec<String>,
}
impl Mcfunction {
    pub fn full_path(&self) -> PathBuf {
        datapack_dir()
            .join("data")
            .join(self.name.namespace())
            .join("functions")
            .join(self.name.path())
            .with_extension("mcfunction")
    }
}

pub fn enable_logging() -> String {
    logged_command("function minect:enable_logging")
}

pub fn reset_logging() -> String {
    logged_command("function minect:reset_logging")
}

pub fn named_logged_command(command: &str) -> String {
    LoggedCommand::builder(command.to_string())
        .name(LISTENER_NAME)
        .build()
        .to_string()
}

pub fn logged_command(command: &str) -> String {
    LoggedCommand::builder(command.to_string())
        .build()
        .to_string()
}

pub fn create_and_enable_datapack(functions: Vec<Mcfunction>) {
    create_datapack(functions);
    enable_debug_datapack();
}

pub fn create_datapack(functions: Vec<Mcfunction>) {
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

pub fn datapack_dir() -> std::path::PathBuf {
    Path::new(TEST_WORLD_DIR)
        .join("datapacks")
        .join(TEST_DATAPACK_NAME)
}

fn enable_debug_datapack() {
    let connection = MinecraftConnection::new(
        "dap".to_string(),
        TEST_WORLD_DIR.into(),
        TEST_LOG_FILE.into(),
    );
    connection
        .inject_commands(vec![
            "function debug:uninstall".to_string(),
            format!("datapack enable \"file/debug-{}\"", TEST_DATAPACK_NAME),
        ])
        .unwrap();
}

pub fn assert_success_response(
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

pub fn assert_error_response(
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

pub fn added_tag_message(tag_name: &str) -> String {
    format!("Added tag '{}' to {}", tag_name, LISTENER_NAME)
}
