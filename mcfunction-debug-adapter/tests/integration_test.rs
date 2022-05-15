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
    added_tag_message, assert_all_breakpoints_verified, assert_error_response,
    create_and_enable_datapack, create_datapack, datapack_dir, enable_logging,
    named_logged_command, reset_logging, start_adapter, Mcfunction, LISTENER_NAME, TEST_LOG_FILE,
};
use assert2::assert;
use debug_adapter_protocol::types::SourceBreakpoint;
use futures::StreamExt;
use mcfunction_debugger::parser::command::resource_location::ResourceLocation;
use minect::log_observer::LogObserver;
use serial_test::serial;
use std::io;
use tokio::sync::mpsc::error::TryRecvError;

#[tokio::test]
#[serial]
async fn test_program_without_breakpoint() -> io::Result<()> {
    let test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            enable_logging(),
            named_logged_command("tag @s add some_tag"),
            reset_logging(),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test]);

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut log_listener = log_observer.add_listener(LISTENER_NAME);
    let mut adapter = start_adapter();
    adapter.initalize().await;
    adapter.launch(&test_path).await;
    adapter.assert_terminated().await;
    assert!(log_listener.recv().await.unwrap().message == added_tag_message("some_tag"));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_program_not_in_data_directory_of_datapack() -> io::Result<()> {
    create_and_enable_datapack(Vec::new());
    let test_path = datapack_dir().join("not-data").join("test.mcfunction");

    let mut adapter = start_adapter();
    adapter.initalize().await;

    let request_seq = adapter.send_launch(&test_path).await;
    let response = adapter.output.next().await.unwrap();
    let error_response = assert_error_response(response, request_seq);
    assert!(error_response.command == "launch");
    assert!(error_response
        .message
        .starts_with("Attribute 'program' does not denote a path in the data directory"));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_breakpoint() -> io::Result<()> {
    let test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            /* 1 */ enable_logging(),
            /* 2 */ named_logged_command("tag @s add tag1"),
            /* 3 */ named_logged_command("tag @s add tag2"),
            /* 4 */ reset_logging(),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test]);

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut log_listener = log_observer.add_listener(LISTENER_NAME);
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(3).build()];
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(log_listener.recv().await.unwrap().message == added_tag_message("tag1")); // First line executed
    assert!(log_listener.try_recv().unwrap_err() == TryRecvError::Empty); // Second line NOT executed
    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(log_listener.recv().await.unwrap().message == added_tag_message("tag2")); // Second line executed
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_breakpoint_at_first_line_of_function() -> io::Result<()> {
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![named_logged_command("tag @s add some_tag")],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            enable_logging(),
            format!("function {}", inner.name),
            reset_logging(),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut log_listener = log_observer.add_listener(LISTENER_NAME);
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(1).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(log_listener.try_recv().unwrap_err() == TryRecvError::Empty);
    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(log_listener.recv().await.unwrap().message == added_tag_message("some_tag"));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_breakpoint_at_function_call() -> io::Result<()> {
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![named_logged_command("tag @s add tag2")],
    };
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ enable_logging(),
            /* 2 */ named_logged_command("tag @s add tag1"),
            /* 3 */ format!("function {}", inner.name),
            /* 4 */ reset_logging(),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut log_listener = log_observer.add_listener(LISTENER_NAME);
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(3).build()];
    adapter.set_breakpoints_verified(&outer_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(log_listener.recv().await.unwrap().message == added_tag_message("tag1")); // First line executed
    assert!(log_listener.try_recv().unwrap_err() == TryRecvError::Empty); // Function NOT executed
    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(log_listener.recv().await.unwrap().message == added_tag_message("tag2")); // Function executed
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_breakpoint_after_launch() -> io::Result<()> {
    let test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            /* 1 */ enable_logging(),
            /* 2 */ named_logged_command("tag @s add tag1"),
            /* 3 */ named_logged_command("tag @s add tag2"),
            /* 4 */ reset_logging(),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test]);

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut log_listener = log_observer.add_listener(LISTENER_NAME);
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let mut breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(log_listener.try_recv().unwrap_err() == TryRecvError::Empty); // First line NOT executed

    breaks.push(SourceBreakpoint::builder().line(3).build());
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.continue_().await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(log_listener.recv().await.unwrap().message == added_tag_message("tag1")); // First line executed
    assert!(log_listener.try_recv().unwrap_err() == TryRecvError::Empty); // Second line NOT executed
    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(log_listener.recv().await.unwrap().message == added_tag_message("tag2")); // Second line executed
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_breakpoint_removed() -> io::Result<()> {
    let test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            /* 1 */ enable_logging(),
            /* 2 */ named_logged_command("tag @s add tag1"),
            /* 3 */ named_logged_command("tag @s add tag2"),
            /* 4 */ reset_logging(),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test]);

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut log_listener = log_observer.add_listener(LISTENER_NAME);
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let mut breaks = vec![
        SourceBreakpoint::builder().line(2).build(),
        SourceBreakpoint::builder().line(3).build(),
    ];
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(log_listener.try_recv().unwrap_err() == TryRecvError::Empty); // First line NOT executed

    breaks.remove(1);
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(log_listener.recv().await.unwrap().message == added_tag_message("tag1"));
    assert!(log_listener.recv().await.unwrap().message == added_tag_message("tag2"));
    Ok(())
}

#[tokio::test]
#[serial]
#[ignore = "https://github.com/vanilla-technologies/mcfunction-debugger/issues/70"]
async fn test_hot_code_replacement() -> io::Result<()> {
    let mut test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            /* 1 */ enable_logging(),
            /* 2 */ named_logged_command("tag @s add tag1"),
            /* 3 */ reset_logging(),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test.clone()]);

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut log_listener = log_observer.add_listener(LISTENER_NAME);
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(3).build()];
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(log_listener.recv().await.unwrap().message == added_tag_message("tag1")); // First line executed
    assert!(log_listener.try_recv().unwrap_err() == TryRecvError::Empty); // Second line NOT executed

    test.lines
        .insert(2, named_logged_command("tag @s add tag2"));
    create_datapack(vec![test]);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(log_listener.recv().await.unwrap().message == added_tag_message("tag2"));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_breakpoint_moved() -> io::Result<()> {
    let mut test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            /* 1 */ enable_logging(),
            /* 2 */ named_logged_command("tag @s add tag1"),
            /* 3 */ named_logged_command("tag @s add tag2"),
            /* 4 */ reset_logging(),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test.clone()]);

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut log_listener = log_observer.add_listener(LISTENER_NAME);
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(3).build()];
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(log_listener.recv().await.unwrap().message == added_tag_message("tag1")); // First line executed
    assert!(log_listener.try_recv().unwrap_err() == TryRecvError::Empty); // Second line NOT executed

    test.lines.remove(1);
    create_datapack(vec![test]);
    let breaks = vec![SourceBreakpoint::builder().line(2).build()];
    let response = adapter
        .set_breakpoints_source_modified(&test_path, &breaks, true)
        .await;
    assert_all_breakpoints_verified(&response, &breaks);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(log_listener.recv().await.unwrap().message == added_tag_message("tag2"));
    Ok(())
}
