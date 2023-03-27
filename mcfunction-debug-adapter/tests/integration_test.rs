// McFunction-Debugger is a debugger for Minecraft's *.mcfunction files that does not require any
// Minecraft mods.
//
// © Copyright (C) 2021-2023 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
//
// This file is part of McFunction-Debugger.
//
// McFunction-Debugger is free software: you can redistribute it and/or modify it under the terms of
// the GNU General Public License as published by the Free Software Foundation, either version 3 of
// the License, or (at your option) any later version.
//
// McFunction-Debugger is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with McFunction-Debugger.
// If not, see <http://www.gnu.org/licenses/>.

mod utils;

use crate::utils::{
    added_tag_output, assert_all_breakpoints_verified, assert_error_response, connection,
    create_and_enable_datapack, create_datapack, datapack_dir, get_source_path,
    named_logged_command, start_adapter,
    timeout::{TimeoutStream, TimeoutStreamError},
    Mcfunction, LISTENER_NAME, TEST_LOG_FILE,
};
use assert2::assert;
use debug_adapter_protocol::types::SourceBreakpoint;
use futures::executor::block_on;
use log::LevelFilter;
use mcfunction_debug_adapter::adapter::SELECTED_ENTITY_SCORES;
use mcfunction_debugger::parser::command::resource_location::ResourceLocation;
use minect::{
    command::{
        add_tag_command, enable_logging_command, logged_command, reset_logging_command,
        summon_named_entity_command, SummonNamedEntityOutput,
    },
    log::LogObserver,
    Command, MinecraftConnection,
};
use serial_test::serial;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::{
    io::{self},
    sync::Once,
    time::Duration,
};
use tokio::time::{sleep, timeout};
use tokio_stream::StreamExt;

fn before_all_tests() {
    TermLogger::init(
        LevelFilter::Trace,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    // If this is the first connection to Minecraft we need to reload to activate the minect datapack.
    let mut connection = connection();
    connection
        .execute_commands([Command::new("reload")])
        .unwrap();
    wait_for_connection(&mut connection);
}

fn wait_for_connection(connection: &mut MinecraftConnection) {
    const INITIAL_CONNECT_ENTITY_NAME: &str = "test_connected";
    const INITIAL_CONNECT_TIMEOUT: Duration = Duration::from_secs(60);
    let events = connection.add_listener();
    connection
        .execute_commands([Command::new(summon_named_entity_command(
            INITIAL_CONNECT_ENTITY_NAME,
        ))])
        .unwrap();
    block_on(timeout(
        INITIAL_CONNECT_TIMEOUT,
        events
            .filter_map(|event| event.output.parse::<SummonNamedEntityOutput>().ok())
            .filter(|output| output.name == INITIAL_CONNECT_ENTITY_NAME)
            .next(),
    ))
    .unwrap();
}

fn before_each_test() {
    static BEFORE_ALL_TESTS: Once = Once::new();
    BEFORE_ALL_TESTS.call_once(before_all_tests);
}

#[tokio::test]
#[serial]
async fn test_program_without_breakpoint() -> io::Result<()> {
    before_each_test();
    let test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            logged_command(enable_logging_command()),
            named_logged_command(add_tag_command("@s", "some_tag")),
            logged_command(reset_logging_command()),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;
    adapter.launch(&test_path).await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("some_tag"));
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_program_not_in_data_directory_of_datapack() -> io::Result<()> {
    before_each_test();
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
    before_each_test();
    let test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 3 */ named_logged_command(add_tag_command("@s", "tag2")),
            /* 4 */ logged_command(reset_logging_command()),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(3).build()];
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // Second line NOT executed
    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_breakpoint_at_first_line_of_function() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![named_logged_command(add_tag_command("@s", "some_tag"))],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            logged_command(enable_logging_command()),
            format!("function {}", inner.name),
            logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(1).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("some_tag"));
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_breakpoint_at_function_call() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![named_logged_command(add_tag_command("@s", "tag2"))],
    };
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 3 */ format!("function {}", inner.name),
            /* 4 */ logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(3).build()];
    adapter.set_breakpoints_verified(&outer_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // Function NOT executed
    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Function executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_breakpoint_after_launch() -> io::Result<()> {
    before_each_test();
    let test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 3 */ named_logged_command(add_tag_command("@s", "tag2")),
            /* 4 */ logged_command(reset_logging_command()),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let mut breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // First line NOT executed

    breaks.push(SourceBreakpoint::builder().line(3).build());
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.continue_().await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // Second line NOT executed
    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_breakpoint_removed() -> io::Result<()> {
    before_each_test();
    let test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 3 */ named_logged_command(add_tag_command("@s", "tag2")),
            /* 4 */ logged_command(reset_logging_command()),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let mut breaks = vec![
        SourceBreakpoint::builder().line(2).build(),
        SourceBreakpoint::builder().line(3).build(),
    ];
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // First line NOT executed

    breaks.remove(1);
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1"));
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2"));
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
#[ignore = "https://github.com/vanilla-technologies/mcfunction-debugger/issues/70"]
async fn test_hot_code_replacement() -> io::Result<()> {
    before_each_test();
    let mut test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 3 */ logged_command(reset_logging_command()),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test.clone()]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(3).build()];
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // Second line NOT executed

    test.lines
        .insert(2, named_logged_command(add_tag_command("@s", "tag2")));
    create_datapack(vec![test]);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2"));
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_breakpoint_moved() -> io::Result<()> {
    before_each_test();
    let mut test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 3 */ named_logged_command(add_tag_command("@s", "tag2")),
            /* 4 */ logged_command(reset_logging_command()),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test.clone()]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(3).build()];
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // Second line NOT executed

    test.lines.remove(1);
    create_datapack(vec![test]);
    let breaks = vec![SourceBreakpoint::builder().line(2).build()];
    let response = adapter
        .set_breakpoints_source_modified(&test_path, &breaks, true)
        .await;
    assert_all_breakpoints_verified(&response, &breaks);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2"));
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_current_breakpoint_removed() -> io::Result<()> {
    before_each_test();
    let test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ named_logged_command(add_tag_command("@s", "some_tag")),
            /* 3 */ logged_command(reset_logging_command()),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let mut breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // First line NOT executed

    breaks.remove(0);
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    sleep(Duration::from_secs(1)).await; // Wait for minecraft to register changed breakpoints

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("some_tag"));
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_current_breakpoint_removed_while_iterating() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ named_logged_command(add_tag_command("@s", "some_tag")),
            /* 3 */ logged_command(reset_logging_command()),
        ],
    };
    let test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            "kill @e[type=sheep,tag=test]".to_string(),
            "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            format!(
                "execute as @e[type=sheep,tag=test] run function {}",
                inner.name
            ),
        ],
    };
    let test_path = test.full_path();
    let inner_path = inner.full_path();
    create_and_enable_datapack(vec![test, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let mut breaks = vec![SourceBreakpoint::builder().line(3).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("some_tag")); // First iteration was executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // Second iteration was NOT executed

    breaks.remove(0);
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    sleep(Duration::from_secs(1)).await; // Wait for minecraft to register changed breakpoints

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("some_tag")); // Second iteration was executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

/// Reproducer for race condition mentioned in https://github.com/vanilla-technologies/mcfunction-debugger/issues/63
#[tokio::test]
#[serial]
async fn test_current_breakpoint_removed_continue_followed_by_set_breakpoints() -> io::Result<()> {
    before_each_test();
    let test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ named_logged_command(add_tag_command("@s", "some_tag")),
            /* 3 */ logged_command(reset_logging_command()),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let mut breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // First line NOT executed

    breaks.remove(0);
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.continue_().await;

    // This runs before minecraft executes debug:resume which originally caused the breakpoint of kind continue to be deleted
    breaks.push(SourceBreakpoint::builder().line(1).build());
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("some_tag"));
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_scope_selected_entity_score() -> io::Result<()> {
    before_each_test();
    const SCOPE: &str = SELECTED_ENTITY_SCORES;

    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ "scoreboard players set @s test_local 42".to_string(),
            /* 2 */ "scoreboard objectives remove test_local".to_string(),
        ],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            "scoreboard objectives add test_local dummy".to_string(),
            "scoreboard players reset * test_local".to_string(),
            "kill @e[type=sheep,tag=test]".to_string(),
            "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            format!(
                "execute as @e[type=sheep,tag=test] run function {}",
                inner.name
            ),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    let vars = adapter.variables_of_scope(stack_trace[0].id, SCOPE).await;
    assert!(vars.len() == 1);
    assert!(vars[0].name == "test_local");
    assert!(vars[0].value == "42");

    adapter.continue_().await;
    adapter.assert_terminated().await;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_scope_selected_entity_score_can_be_removed() -> io::Result<()> {
    before_each_test();
    const SCOPE: &str = SELECTED_ENTITY_SCORES;

    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ "scoreboard players set @s test_local 42".to_string(),
            /* 2 */ "scoreboard players reset @s test_local".to_string(),
            /* 3 */ "scoreboard objectives remove test_local".to_string(),
        ],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            "scoreboard objectives add test_local dummy".to_string(),
            "scoreboard players reset * test_local".to_string(),
            "kill @e[type=sheep,tag=test]".to_string(),
            "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            format!(
                "execute as @e[type=sheep,tag=test] run function {}",
                inner.name
            ),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = [
        SourceBreakpoint::builder().line(2).build(),
        SourceBreakpoint::builder().line(3).build(),
    ];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    let vars = adapter.variables_of_scope(stack_trace[0].id, SCOPE).await;
    assert!(vars.len() == 1);
    assert!(vars[0].name == "test_local");
    assert!(vars[0].value == "42");

    adapter.continue_().await;
    adapter.assert_stopped_at_breakpoint().await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    let vars = adapter.variables_of_scope(stack_trace[0].id, SCOPE).await;
    assert!(vars.is_empty());

    adapter.continue_().await;
    adapter.assert_terminated().await;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_scope_selected_entity_score_multiple_depths() -> io::Result<()> {
    before_each_test();
    const SCOPE: &str = SELECTED_ENTITY_SCORES;

    let pig = Mcfunction {
        name: ResourceLocation::new("adapter_test", "pig"),
        lines: vec![
            /* 1 */ "scoreboard players set @s test_local 52".to_string(),
            /* 2 */ "scoreboard objectives remove test_local".to_string(),
        ],
    };
    let pig_path = pig.full_path();
    let sheep = Mcfunction {
        name: ResourceLocation::new("adapter_test", "sheep"),
        lines: vec![
            /* 1 */ "scoreboard players set @s test_local 42".to_string(),
            /* 2 */ "kill @e[type=pig,tag=test]".to_string(),
            /* 3 */ "summon pig ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            /* 4 */ format!("execute as @e[type=pig,tag=test] run function {}", pig.name),
        ],
    };
    let test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            "scoreboard objectives add test_local dummy".to_string(),
            "scoreboard players reset * test_local".to_string(),
            "kill @e[type=sheep,tag=test]".to_string(),
            "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            format!(
                "execute as @e[type=sheep,tag=test] run function {}",
                sheep.name
            ),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test, sheep, pig]);

    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = [SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&pig_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 3);

    let vars = adapter.variables_of_scope(stack_trace[0].id, SCOPE).await;
    assert!(vars.len() == 1);
    assert!(vars[0].name == "test_local");
    assert!(vars[0].value == "52");

    let vars = adapter.variables_of_scope(stack_trace[1].id, SCOPE).await;
    assert!(vars.len() == 1);
    assert!(vars[0].name == "test_local");
    assert!(vars[0].value == "42");

    let scopes = adapter.scopes(stack_trace[2].id).await;
    assert!(scopes.iter().find(|it| it.name == SCOPE).is_none());

    adapter.continue_().await;
    adapter.assert_terminated().await;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_scope_selected_entity_score_server_context() -> io::Result<()> {
    before_each_test();
    const SCOPE: &str = SELECTED_ENTITY_SCORES;

    let test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            /* 1 */ "scoreboard objectives add test_local dummy".to_string(),
            /* 2 */ "scoreboard players reset * test_local".to_string(),
            /* 3 */ "scoreboard players set @s test_local 42".to_string(),
            /* 4 */ "scoreboard objectives remove test_local".to_string(),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test]);

    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(4).build()];
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    let scopes = adapter.scopes(stack_trace[0].id).await;
    assert!(scopes.iter().find(|it| it.name == SCOPE).is_none());

    adapter.continue_().await;
    adapter.assert_terminated().await;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_step_out_of_root_function() -> io::Result<()> {
    before_each_test();
    let test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ named_logged_command(add_tag_command("@s", "some_tag")),
            /* 3 */ logged_command(reset_logging_command()),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // Line NOT executed

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_out(threads[0].id).await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("some_tag")); // Line was executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_step_out_of_inner_function() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag2")),
        ],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            logged_command(enable_logging_command()),
            format!("function {}", inner.name),
            named_logged_command(add_tag_command("@s", "tag3")),
            logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(1).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_out(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // Third line NOT executed

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag3")); // Third line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_step_out_of_inner_function_with_multiple_executors() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag2")),
        ],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            "kill @e[type=sheep,tag=test]".to_string(),
            "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            logged_command(enable_logging_command()),
            format!(
                "execute as @e[type=sheep,tag=test] run function {}",
                inner.name
            ),
            named_logged_command(add_tag_command("@s", "tag3")),
            logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let mut breaks = vec![SourceBreakpoint::builder().line(1).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    breaks.remove(0);
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_out(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed by first sheep
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed by first sheep
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed by second sheep
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed by second sheep
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // Third line NOT executed

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag3")); // Third line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_step_out_of_inner_function_with_breakpoint() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag2")),
        ],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            logged_command(enable_logging_command()),
            format!("function {}", inner.name),
            named_logged_command(add_tag_command("@s", "tag3")),
            logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![
        SourceBreakpoint::builder().line(1).build(),
        SourceBreakpoint::builder().line(2).build(),
    ];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_out(threads[0].id).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // Second line NOT executed

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed
    assert!(listener.next().await.unwrap().output == added_tag_output("tag3")); // Third line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_step_out_into_end_of_function() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 3 */ logged_command(reset_logging_command()),
        ],
    };
    let inner_path = inner.full_path();
    let outer_line = format!("function {}", inner.name);
    let outer_line_len = outer_line.len();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![outer_line],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(1).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_out(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 1);
    assert!(stack_trace[0].line == 1);
    assert!(stack_trace[0].column == outer_line_len as i32 + 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_step_out_into_end_of_function_with_breakpoint() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 3 */ logged_command(reset_logging_command()),
        ],
    };
    let inner_path = inner.full_path();
    let outer_line = format!("function {}", inner.name);
    let outer_line_len = outer_line.len();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![outer_line],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(1).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let breaks = vec![SourceBreakpoint::builder().line(1).build()];
    adapter.set_breakpoints_verified(&outer_path, &breaks).await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_out(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 1);
    assert!(stack_trace[0].line == 1);
    assert!(stack_trace[0].column == outer_line_len as i32 + 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_step_out_into_start_of_function_with_breakpoint_via_recursion() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 3 */ logged_command(reset_logging_command()),
            /* 4 */ "scoreboard players set test test_global 1".to_string(),
            /* 5 */ "function adapter_test:outer".to_string(),
        ],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ "scoreboard objectives add test_global dummy".to_string(),
            /* 2 */
            format!(
                "execute unless score test test_global matches 1 run function {}",
                inner.name
            ),
            /* 3 */ "scoreboard objectives remove test_global".to_string(),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(1).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&outer_path, &breaks).await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_out(threads[0].id).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 3);
    assert!(get_source_path(&stack_trace[0]) == &outer_path.display().to_string());
    assert!(stack_trace[0].line == 2);
    assert!(stack_trace[0].column == 1);
    assert!(get_source_path(&stack_trace[1]) == &inner_path.display().to_string());
    assert!(stack_trace[1].line == 5);
    assert!(stack_trace[1].column == 1);
    assert!(get_source_path(&stack_trace[2]) == &outer_path.display().to_string());
    assert!(stack_trace[2].line == 2);
    assert!(stack_trace[2].column == 1);

    let breaks = Vec::new();
    adapter.set_breakpoints_verified(&outer_path, &breaks).await;

    let breaks = Vec::new();
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.continue_().await;
    adapter.assert_terminated().await;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_step_out_of_inner_function_that_recursively_calls_outer_function() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 3 */ logged_command(reset_logging_command()),
            /* 4 */ "scoreboard players set test test_global 1".to_string(),
            /* 5 */ "function adapter_test:outer".to_string(),
        ],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ "scoreboard objectives add test_global dummy".to_string(),
            /* 2 */
            format!(
                "execute unless score test test_global matches 1 run function {}",
                inner.name
            ),
            /* 3 */ "scoreboard objectives remove test_global".to_string(),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(1).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let breaks = vec![];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_out(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 1);
    assert!(get_source_path(&stack_trace[0]) == &outer_path.display().to_string());
    assert!(stack_trace[0].line == 3);
    assert!(stack_trace[0].column == 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    Ok(())
}

/// This was a bug caused by a resumption point and a step point at the same position
#[tokio::test]
#[serial]
async fn test_step_out_of_recursive_function_to_same_position() -> io::Result<()> {
    before_each_test();
    let inner_name = ResourceLocation::new("adapter_test", "inner");
    let inner = Mcfunction {
        lines: vec![
            /* 1 */ "scoreboard players remove test test_global 1".to_string(),
            /* 2 */
            format!(
                "execute if score test test_global matches 1.. run function {}",
                inner_name
            ),
            /* 3 */ "scoreboard players reset test test_global".to_string(),
        ],
        name: inner_name,
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ "scoreboard objectives add test_global dummy".to_string(),
            /* 2 */ "scoreboard players set test test_global 2".to_string(),
            /* 3 */ format!("function {}", inner.name),
            /* 4 */ "scoreboard objectives remove test_global".to_string(),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(3).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 3);
    assert!(get_source_path(&stack_trace[0]) == &inner_path.display().to_string());
    assert!(stack_trace[0].line == 3);
    assert!(stack_trace[0].column == 1);
    assert!(get_source_path(&stack_trace[1]) == &inner_path.display().to_string());
    assert!(stack_trace[1].line == 2);
    assert!(stack_trace[1].column == 1);
    assert!(get_source_path(&stack_trace[2]) == &outer_path.display().to_string());
    assert!(stack_trace[2].line == 3);
    assert!(stack_trace[2].column == 1);

    let breaks = vec![];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_out(threads[0].id).await;
    adapter.assert_stopped_after_step().await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 2);
    assert!(get_source_path(&stack_trace[0]) == &inner_path.display().to_string());
    assert!(stack_trace[0].line == 3);
    assert!(stack_trace[0].column == 1);
    assert!(get_source_path(&stack_trace[1]) == &outer_path.display().to_string());
    assert!(stack_trace[1].line == 3);
    assert!(stack_trace[1].column == 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_next_steps_over_command() -> io::Result<()> {
    before_each_test();
    let test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 3 */ named_logged_command(add_tag_command("@s", "tag2")),
            /* 4 */ logged_command(reset_logging_command()),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // First line NOT executed

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.next(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // Second line NOT executed

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_next_steps_over_function() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            named_logged_command(add_tag_command("@s", "tag1")),
            named_logged_command(add_tag_command("@s", "tag2")),
        ],
    };
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ format!("function {}", inner.name),
            /* 3 */ named_logged_command(add_tag_command("@s", "tag3")),
            /* 4 */ logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&outer_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // First line NOT executed

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.next(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // Third line NOT executed

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag3")); // Third line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_next_steps_over_function_that_recursively_calls_current_function() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag2")),
            /* 3 */ "scoreboard players set test test_global 1".to_string(),
            /* 4 */ "function adapter_test:outer".to_string(),
        ],
    };
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ "scoreboard objectives add test_global dummy".to_string(),
            /* 2 */ logged_command(enable_logging_command()),
            /* 3 */
            format!(
                "execute unless score test test_global matches 1 run function {}",
                inner.name
            ),
            /* 4 */ named_logged_command(add_tag_command("@s", "tag3")),
            /* 5 */ logged_command(reset_logging_command()),
            /* 6 */ "scoreboard objectives remove test_global".to_string(),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(3).build()];
    adapter.set_breakpoints_verified(&outer_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // First line NOT executed

    let breaks = vec![];
    adapter.set_breakpoints_verified(&outer_path, &breaks).await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.next(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed
    assert!(listener.next().await.unwrap().output == added_tag_output("tag3")); // Third line executed via recursion
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 1);
    assert!(get_source_path(&stack_trace[0]) == &outer_path.display().to_string());
    assert!(stack_trace[0].line == 4);
    assert!(stack_trace[0].column == 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag3")); // Third line executed normally
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_next_steps_into_function_with_breakpoint() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag2")),
        ],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ format!("function {}", inner.name),
            /* 3 */ named_logged_command(add_tag_command("@s", "tag3")),
            /* 4 */ logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&outer_path, &breaks).await;

    let breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // First line NOT executed

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.next(threads[0].id).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // Second line NOT executed

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 2);
    assert!(get_source_path(&stack_trace[0]) == &inner_path.display().to_string());
    assert!(stack_trace[0].line == 2);
    assert!(stack_trace[0].column == 1);
    assert!(get_source_path(&stack_trace[1]) == &outer_path.display().to_string());
    assert!(stack_trace[1].line == 2);
    assert!(stack_trace[1].column == 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed
    assert!(listener.next().await.unwrap().output == added_tag_output("tag3")); // Third line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_next_steps_out_of_function() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ named_logged_command(add_tag_command("@s", "tag1")),
        ],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ format!("function {}", inner.name),
            /* 3 */ named_logged_command(add_tag_command("@s", "tag2")),
            /* 4 */ logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(1).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // First line NOT executed

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.next(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // Second line NOT executed

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 1);
    assert!(get_source_path(&stack_trace[0]) == &outer_path.display().to_string());
    assert!(stack_trace[0].line == 3);
    assert!(stack_trace[0].column == 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_next_steps_into_next_executor() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag2")),
        ],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ "kill @e[type=sheep,tag=test]".to_string(),
            /* 2 */ "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            /* 3 */ "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            /* 4 */ logged_command(enable_logging_command()),
            /* 5 */
            format!(
                "execute as @e[type=sheep,tag=test] run function {}",
                inner.name
            ),
            /* 6 */ named_logged_command(add_tag_command("@s", "tag3")),
            /* 7 */ logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed by first sheep
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let breaks = vec![];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.next(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed by first sheep
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 2);
    assert!(get_source_path(&stack_trace[0]) == &inner_path.display().to_string());
    assert!(stack_trace[0].line == 1);
    assert!(stack_trace[0].column == 1);
    assert!(get_source_path(&stack_trace[1]) == &outer_path.display().to_string());
    assert!(stack_trace[1].line == 5);
    assert!(stack_trace[1].column == 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed by second sheep
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed by second sheep
    assert!(listener.next().await.unwrap().output == added_tag_output("tag3")); // Third line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_next_steps_into_next_empty_executor() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ "kill @e[type=sheep,tag=test]".to_string(),
            /* 2 */ "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            /* 3 */ "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            /* 4 */ logged_command(enable_logging_command()),
            /* 5 */
            format!(
                "execute as @e[type=sheep,tag=test] run function {}",
                inner.name
            ),
            /* 6 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 7 */ logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(5).build()];
    adapter.set_breakpoints_verified(&outer_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_in(threads[0].id).await;
    adapter.assert_stopped_after_step().await; // Stopped in empty function for sheep 1
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 2);
    assert!(get_source_path(&stack_trace[0]) == &inner_path.display().to_string());
    assert!(stack_trace[0].line == 1);
    assert!(stack_trace[0].column == 1);
    assert!(get_source_path(&stack_trace[1]) == &outer_path.display().to_string());
    assert!(stack_trace[1].line == 5);
    assert!(stack_trace[1].column == 1);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.next(threads[0].id).await;
    adapter.assert_stopped_after_step().await; // Stopped in empty function for sheep 2
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 2);
    assert!(get_source_path(&stack_trace[0]) == &inner_path.display().to_string());
    assert!(stack_trace[0].line == 1);
    assert!(stack_trace[0].column == 1);
    assert!(get_source_path(&stack_trace[1]) == &outer_path.display().to_string());
    assert!(stack_trace[1].line == 5);
    assert!(stack_trace[1].column == 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1"));
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_next_steps_into_next_executor_skipping_non_commands() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ "".to_string(), // skipped
            /* 2 */ "# comment".to_string(), // skipped
            /* 3 */ named_logged_command(add_tag_command("@s", "tag1")),
        ],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ "kill @e[type=sheep,tag=test]".to_string(),
            /* 2 */ "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            /* 3 */ "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            /* 4 */ logged_command(enable_logging_command()),
            /* 5 */
            format!(
                "execute as @e[type=sheep,tag=test] run function {}",
                inner.name
            ),
            /* 6 */ named_logged_command(add_tag_command("@s", "tag2")),
            /* 7 */ logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(3).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let breaks = vec![];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.next(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // Executed by first sheep
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 2);
    assert!(get_source_path(&stack_trace[0]) == &inner_path.display().to_string());
    assert!(stack_trace[0].line == 3);
    assert!(stack_trace[0].column == 1);
    assert!(get_source_path(&stack_trace[1]) == &outer_path.display().to_string());
    assert!(stack_trace[1].line == 5);
    assert!(stack_trace[1].column == 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // Executed by second sheep
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2"));
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_step_in_steps_over_command() -> io::Result<()> {
    before_each_test();
    let test = Mcfunction {
        name: ResourceLocation::new("adapter_test", "test"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 3 */ named_logged_command(add_tag_command("@s", "tag2")),
            /* 4 */ logged_command(reset_logging_command()),
        ],
    };
    let test_path = test.full_path();
    create_and_enable_datapack(vec![test]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&test_path, &breaks).await;

    adapter.launch(&test_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // First line NOT executed

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_in(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // Second line NOT executed

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_step_in_function() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![named_logged_command(add_tag_command("@s", "tag1"))],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ format!("function {}", inner.name),
            /* 3 */ named_logged_command(add_tag_command("@s", "tag2")),
            /* 4 */ logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&outer_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // First line NOT executed

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_in(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // First line NOT executed

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 2);
    assert!(get_source_path(&stack_trace[0]) == &inner_path.display().to_string());
    assert!(stack_trace[0].line == 1);
    assert!(stack_trace[0].column == 1);
    assert!(get_source_path(&stack_trace[1]) == &outer_path.display().to_string());
    assert!(stack_trace[1].line == 2);
    assert!(stack_trace[1].column == 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1"));
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2"));
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_step_in_empty_function() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ format!("function {}", inner.name),
            /* 3 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 4 */ logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&outer_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_in(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 2);
    assert!(get_source_path(&stack_trace[0]) == &inner_path.display().to_string());
    assert!(stack_trace[0].line == 1);
    assert!(stack_trace[0].column == 1);
    assert!(get_source_path(&stack_trace[1]) == &outer_path.display().to_string());
    assert!(stack_trace[1].line == 2);
    assert!(stack_trace[1].column == 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1"));
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_step_in_steps_over_invalid_function() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            "this is an invalid command".to_string(),
            named_logged_command(add_tag_command("@s", "tag1")),
        ],
    };
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ format!("function {}", inner.name),
            /* 3 */ named_logged_command(add_tag_command("@s", "tag2")),
            /* 4 */ logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&outer_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_in(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 1);
    assert!(get_source_path(&stack_trace[0]) == &outer_path.display().to_string());
    assert!(stack_trace[0].line == 3);
    assert!(stack_trace[0].column == 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // tag1 is skipped
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_step_in_steps_out_of_function() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ named_logged_command(add_tag_command("@s", "tag1")),
        ],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ logged_command(enable_logging_command()),
            /* 2 */ format!("function {}", inner.name),
            /* 3 */ named_logged_command(add_tag_command("@s", "tag2")),
            /* 4 */ logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(1).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // First line NOT executed

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_in(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout); // Second line NOT executed

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 1);
    assert!(get_source_path(&stack_trace[0]) == &outer_path.display().to_string());
    assert!(stack_trace[0].line == 3);
    assert!(stack_trace[0].column == 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_step_in_next_executor() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 2 */ named_logged_command(add_tag_command("@s", "tag2")),
        ],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ "kill @e[type=sheep,tag=test]".to_string(),
            /* 2 */ "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            /* 3 */ "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            /* 4 */ logged_command(enable_logging_command()),
            /* 5 */
            format!(
                "execute as @e[type=sheep,tag=test] run function {}",
                inner.name
            ),
            /* 6 */ named_logged_command(add_tag_command("@s", "tag3")),
            /* 7 */ logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(2).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed by first sheep
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let breaks = vec![];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_in(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed by first sheep
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 2);
    assert!(get_source_path(&stack_trace[0]) == &inner_path.display().to_string());
    assert!(stack_trace[0].line == 1);
    assert!(stack_trace[0].column == 1);
    assert!(get_source_path(&stack_trace[1]) == &outer_path.display().to_string());
    assert!(stack_trace[1].line == 5);
    assert!(stack_trace[1].column == 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // First line executed by second sheep
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2")); // Second line executed by second sheep
    assert!(listener.next().await.unwrap().output == added_tag_output("tag3")); // Third line executed
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_step_in_next_empty_executor() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ "kill @e[type=sheep,tag=test]".to_string(),
            /* 2 */ "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            /* 3 */ "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            /* 4 */ logged_command(enable_logging_command()),
            /* 5 */
            format!(
                "execute as @e[type=sheep,tag=test] run function {}",
                inner.name
            ),
            /* 6 */ named_logged_command(add_tag_command("@s", "tag1")),
            /* 7 */ logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(5).build()];
    adapter.set_breakpoints_verified(&outer_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_in(threads[0].id).await;
    adapter.assert_stopped_after_step().await; // Stopped in empty function for sheep 1
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 2);
    assert!(get_source_path(&stack_trace[0]) == &inner_path.display().to_string());
    assert!(stack_trace[0].line == 1);
    assert!(stack_trace[0].column == 1);
    assert!(get_source_path(&stack_trace[1]) == &outer_path.display().to_string());
    assert!(stack_trace[1].line == 5);
    assert!(stack_trace[1].column == 1);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_in(threads[0].id).await;
    adapter.assert_stopped_after_step().await; // Stopped in empty function for sheep 2
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 2);
    assert!(get_source_path(&stack_trace[0]) == &inner_path.display().to_string());
    assert!(stack_trace[0].line == 1);
    assert!(stack_trace[0].column == 1);
    assert!(get_source_path(&stack_trace[1]) == &outer_path.display().to_string());
    assert!(stack_trace[1].line == 5);
    assert!(stack_trace[1].column == 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1"));
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_step_in_steps_into_next_executor_skipping_non_commands() -> io::Result<()> {
    before_each_test();
    let inner = Mcfunction {
        name: ResourceLocation::new("adapter_test", "inner"),
        lines: vec![
            /* 1 */ "".to_string(), // skipped
            /* 2 */ "# comment".to_string(), // skipped
            /* 3 */ named_logged_command(add_tag_command("@s", "tag1")),
        ],
    };
    let inner_path = inner.full_path();
    let outer = Mcfunction {
        name: ResourceLocation::new("adapter_test", "outer"),
        lines: vec![
            /* 1 */ "kill @e[type=sheep,tag=test]".to_string(),
            /* 2 */ "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            /* 3 */ "summon sheep ~ ~ ~ {Tags: [test], NoAI: true}".to_string(),
            /* 4 */ logged_command(enable_logging_command()),
            /* 5 */
            format!(
                "execute as @e[type=sheep,tag=test] run function {}",
                inner.name
            ),
            /* 6 */ named_logged_command(add_tag_command("@s", "tag2")),
            /* 7 */ logged_command(reset_logging_command()),
        ],
    };
    let outer_path = outer.full_path();
    create_and_enable_datapack(vec![outer, inner]);

    let log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::new(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;

    let breaks = vec![SourceBreakpoint::builder().line(3).build()];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    adapter.launch(&outer_path).await;
    adapter.assert_stopped_at_breakpoint().await;
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let breaks = vec![];
    adapter.set_breakpoints_verified(&inner_path, &breaks).await;

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    adapter.step_in(threads[0].id).await;
    adapter.assert_stopped_after_step().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // Executed by first sheep
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);

    let threads = adapter.threads().await;
    assert!(threads.len() == 1);
    let stack_trace = adapter.stack_trace(threads[0].id).await;
    assert!(stack_trace.len() == 2);
    assert!(get_source_path(&stack_trace[0]) == &inner_path.display().to_string());
    assert!(stack_trace[0].line == 3);
    assert!(stack_trace[0].column == 1);
    assert!(get_source_path(&stack_trace[1]) == &outer_path.display().to_string());
    assert!(stack_trace[1].line == 5);
    assert!(stack_trace[1].column == 1);

    adapter.continue_().await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("tag1")); // Executed by second sheep
    assert!(listener.next().await.unwrap().output == added_tag_output("tag2"));
    assert!(listener.try_next().unwrap_err() == TimeoutStreamError::Timeout);
    Ok(())
}
