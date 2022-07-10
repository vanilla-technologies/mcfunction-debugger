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

mod utils;

use crate::utils::{
    assert_error_response, create_datapack, datapack_dir, enable_logging, named_logged_command,
    reset_logging, start_adapter, Mcfunction, LISTENER_NAME, TEST_LOG_FILE,
};
use assert2::{assert, let_assert};
use debug_adapter_protocol::{
    events::{Event, StoppedEventReason},
    types::{Breakpoint, SourceBreakpoint},
    ProtocolMessageContent as Content,
};
use futures::StreamExt;
use mcfunction_debugger::parser::command::resource_location::ResourceLocation;
use minect::log_observer::LogObserver;
use std::io;
use tokio::sync::mpsc::error::TryRecvError;

#[tokio::test]
async fn test_program_is_executed() -> io::Result<()> {
    let test = Mcfunction {
        name: ResourceLocation::new("test", "bla"),
        lines: vec![
            enable_logging(),
            named_logged_command("tag @s add some_tag"),
            reset_logging(),
        ],
    };
    let test_fn_path = test.full_path();
    create_datapack(vec![test]);

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut log_listener = log_observer.add_listener(LISTENER_NAME);

    let mut adapter = start_adapter();
    adapter.initalize().await;
    adapter.launch(&test_fn_path).await;

    let event = adapter.output.next().await.unwrap();
    assert!(let Content::Event(Event::Terminated(_)) = event.content);

    let log_event = log_listener.recv().await.unwrap();
    assert!(log_event.message == "Added tag 'some_tag' to test");

    adapter.handle.await.unwrap().unwrap();
    Ok(())
}

#[tokio::test]
async fn test_program_not_in_data_directory_of_datapack() -> io::Result<()> {
    create_datapack(Vec::new());
    let test_fn_path = datapack_dir().join("not-data").join("test.mcfunction");

    let mut adapter = start_adapter();
    adapter.initalize().await;

    let request_seq = adapter.send_launch(&test_fn_path).await;
    let response = adapter.output.next().await.unwrap();
    let error_response = assert_error_response(response, request_seq);
    assert!(error_response.command == "launch");
    assert!(error_response
        .message
        .starts_with("Attribute 'program' does not denote a path in the data directory"));
    Ok(())
}

#[tokio::test]
async fn test_breakpoint_suspends_execution() -> io::Result<()> {
    let test = Mcfunction {
        name: ResourceLocation::new("test", "bla"),
        lines: vec![
            /* 1 */ enable_logging(),
            /* 2 */ named_logged_command("tag @s add tag1"),
            /* 3 */ named_logged_command("tag @s add tag2"),
            /* 4 */ reset_logging(),
        ],
    };
    let test_fn_path = test.full_path();
    create_datapack(vec![test]);

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut log_listener = log_observer.add_listener(LISTENER_NAME);

    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breakpoints = vec![SourceBreakpoint::builder().line(3).build()];
    let response = adapter.set_breakpoints(&test_fn_path, breakpoints).await;
    assert!(let [Breakpoint {
      verified: true,
      line: Some(3),
      ..
    }] = response.breakpoints.as_slice());

    adapter.launch(&test_fn_path).await;

    let event = adapter.output.next().await.unwrap();
    let_assert!(Content::Event(Event::Stopped(body)) = event.content);
    assert!(body.reason == StoppedEventReason::Breakpoint);

    // TODO: threads and stacktrace

    // First line executed
    assert!(log_listener.recv().await.unwrap().message == "Added tag 'tag1' to test");

    // Second line NOT executed
    assert!(log_listener.try_recv().unwrap_err() == TryRecvError::Empty);

    adapter.continue_().await;

    // Second line executed
    assert!(log_listener.recv().await.unwrap().message == "Added tag 'tag2' to test");

    let event = adapter.output.next().await.unwrap();
    assert!(let Content::Event(Event::Terminated(_)) = event.content);

    adapter.handle.await.unwrap().unwrap();
    Ok(())
}
