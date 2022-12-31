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

mod utils;

use crate::utils::{
    added_tag_output, assert_all_breakpoints_verified, assert_error_response,
    create_and_enable_datapack, create_datapack, datapack_dir, named_logged_command, start_adapter,
    timeout::{TimeoutStream, TimeoutStreamError},
    Mcfunction, LISTENER_NAME, TEST_LOG_FILE,
};
use assert2::assert;
use debug_adapter_protocol::types::SourceBreakpoint;
use log::LevelFilter;
use mcfunction_debug_adapter::adapter::SELECTED_ENTITY_SCORES;
use mcfunction_debugger::parser::command::resource_location::ResourceLocation;
use minect::log::{
    add_tag_command, enable_logging_command, logged_command, observer::LogObserver,
    reset_logging_command,
};
use serial_test::serial;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::{io, time::Duration};
use tokio::time::sleep;

fn before_test() {
    let _ = TermLogger::init(
        LevelFilter::Trace,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    );
}

#[tokio::test]
#[serial]
async fn test_program_without_breakpoint() -> io::Result<()> {
    before_test();
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

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::from_receiver(log_observer.add_named_listener(LISTENER_NAME));
    let mut adapter = start_adapter();
    adapter.initalize().await;
    adapter.launch(&test_path).await;
    adapter.assert_terminated().await;
    assert!(listener.next().await.unwrap().output == added_tag_output("some_tag"));
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_program_not_in_data_directory_of_datapack() -> io::Result<()> {
    before_test();
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
    before_test();
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

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::from_receiver(log_observer.add_named_listener(LISTENER_NAME));
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
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_breakpoint_at_first_line_of_function() -> io::Result<()> {
    before_test();
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

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::from_receiver(log_observer.add_named_listener(LISTENER_NAME));
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
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_breakpoint_at_function_call() -> io::Result<()> {
    before_test();
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

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::from_receiver(log_observer.add_named_listener(LISTENER_NAME));
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
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_breakpoint_after_launch() -> io::Result<()> {
    before_test();
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

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::from_receiver(log_observer.add_named_listener(LISTENER_NAME));
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
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_breakpoint_removed() -> io::Result<()> {
    before_test();
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

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::from_receiver(log_observer.add_named_listener(LISTENER_NAME));
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
    Ok(())
}

#[tokio::test]
#[serial]
#[ignore = "https://github.com/vanilla-technologies/mcfunction-debugger/issues/70"]
async fn test_hot_code_replacement() -> io::Result<()> {
    before_test();
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

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::from_receiver(log_observer.add_named_listener(LISTENER_NAME));
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
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_breakpoint_moved() -> io::Result<()> {
    before_test();
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

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::from_receiver(log_observer.add_named_listener(LISTENER_NAME));
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
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_current_breakpoint_removed() -> io::Result<()> {
    before_test();
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

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::from_receiver(log_observer.add_named_listener(LISTENER_NAME));
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
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_current_breakpoint_removed_while_iterating() -> io::Result<()> {
    before_test();
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

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::from_receiver(log_observer.add_named_listener(LISTENER_NAME));
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
    Ok(())
}

/// Reproducer for race condition mentioned in https://github.com/vanilla-technologies/mcfunction-debugger/issues/63
#[tokio::test]
#[serial]
async fn test_current_breakpoint_removed_continue_followed_by_set_breakpoints() -> io::Result<()> {
    before_test();
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

    let mut log_observer = LogObserver::new(TEST_LOG_FILE);
    let mut listener = TimeoutStream::from_receiver(log_observer.add_named_listener(LISTENER_NAME));
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
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_scope_selected_entity_score() -> io::Result<()> {
    before_test();
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
    before_test();
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
    before_test();
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
    before_test();
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
