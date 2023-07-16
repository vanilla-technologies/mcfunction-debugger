use futures::{
    future::{select, Either},
    pin_mut,
};
use mcfunction_debugger::{
    adapter::utils::StoppedEvent,
    generator::{
        config::{
            adapter::{AdapterConfig, BreakpointPositionInLine, LocalBreakpointPosition},
            Config,
        },
        generate_debug_datapack,
        parser::command::resource_location::{ResourceLocation, ResourceLocationRef},
        DebugDatapackMetadata,
    },
};
use minect::{
    command::{named_logged_block_commands, summon_named_entity_command, SummonNamedEntityOutput},
    log::LogEvent,
    Command, MinecraftConnection,
};
use serial_test::serial;
use simple_logger::SimpleLogger;
use std::{
    collections::HashMap,
    ffi::OsStr,
    io::{self},
    path::Path,
    sync::atomic::{AtomicBool, AtomicI8, Ordering},
    time::Duration,
};
use tokio::{
    fs::{copy, create_dir, create_dir_all, read_to_string, write},
    sync::OnceCell,
    time::{error::Elapsed, timeout},
    try_join,
};
use tokio_stream::{Stream, StreamExt};
use walkdir::WalkDir;

macro_rules! include_template {
    ( $path:expr) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/datapacks/",
            $path
        ))
    };
}

macro_rules! expand_template {
    ($path:expr, $expand:expr) => {{
        let expand = $expand;
        create_file_owned(
            Path::new(TEST_WORLD_DIR)
                .join("datapacks")
                .join(expand($path)),
            expand(include_template!($path)),
        )
    }};
}

async fn create_file_owned(path: impl AsRef<Path>, contents: String) -> io::Result<()> {
    create_file(path, &contents).await
}

async fn create_file(path: impl AsRef<Path>, contents: &str) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        create_dir_all(parent).await?;
    }
    write(path, contents).await
}

macro_rules! expand_test_template {
    ($path:expr) => {
        expand_template!($path, expand_logged_cmds)
    };
}

fn expand_logged_cmds(string: &str) -> String {
    if !string.contains('\n') {
        return string.to_string();
    }

    let mut expanded = String::with_capacity(string.len());
    for line in string.lines() {
        if let Some((index, executor, command)) = find_special_say_command(line) {
            let prefix = &line[..index];
            let commands = named_logged_block_commands(executor, command);
            for command in commands {
                expanded.push_str(prefix);
                expanded.push_str(&command);
                expanded.push('\n');
            }
        } else {
            expanded.push_str(&line);
        }
        expanded.push('\n');
    }
    expanded
}

fn find_special_say_command(line: &str) -> Option<(usize, &str, &str)> {
    let prefix = "say [";
    let index = line.find(prefix)?;
    let without_closing_bracket = line.strip_suffix(']')?;
    let content = &without_closing_bracket[index + prefix.len()..];
    let (executor, command) = content.split_once(": ")?;
    Some((index, executor, command))
}

macro_rules! include_test_category {
    ($category:expr) => {
        include!(concat!(env!("OUT_DIR"), "/tests/", $category, ".rs"));
    };
}

mod minecraft {
    use super::*;

    mod before_age_increment {
        use super::*;

        macro_rules! test {
            ($namespace:ident, $name:ident) => {
                #[tokio::test]
                #[serial]
                async fn $name() -> io::Result<()> {
                    run_test(stringify!($namespace), stringify!($name), false, false).await
                }
            };
        }

        include_test_category!("test");
        mod minecraft_1_15_plus {
            use super::*;
            include_test_category!("test_1_15_plus");
        }
        include_test_category!("test_before_age_increment");
    }

    mod after_age_increment {
        use super::*;

        macro_rules! test {
            ($namespace:ident, $name:ident) => {
                #[tokio::test]
                #[serial]
                async fn $name() -> io::Result<()> {
                    run_test(stringify!($namespace), stringify!($name), true, false).await
                }
            };
        }

        include_test_category!("test");
        mod minecraft_1_15_plus {
            use super::*;
            include_test_category!("test_1_15_plus");
        }
        include_test_category!("test_after_age_increment");
    }
}

mod debugger {
    use super::*;

    mod before_age_increment {
        use super::*;

        macro_rules! test {
            ($namespace:ident, $name:ident) => {
                #[tokio::test]
                #[serial]
                async fn $name() -> io::Result<()> {
                    run_test(stringify!($namespace), stringify!($name), false, true).await
                }
            };
        }
        include_test_category!("test");
        mod minecraft_1_15_plus {
            use super::*;
            include_test_category!("test_1_15_plus");
        }
        include_test_category!("test_before_age_increment");
    }

    mod after_age_increment {
        use super::*;

        macro_rules! test {
            ($namespace:ident, $name:ident) => {
                #[tokio::test]
                #[serial]
                async fn $name() -> io::Result<()> {
                    run_test(stringify!($namespace), stringify!($name), true, true).await
                }
            };
        }
        include_test_category!("test");
        mod minecraft_1_15_plus {
            use super::*;
            include_test_category!("test_1_15_plus");
        }
        include_test_category!("test_after_age_increment");
    }
}

async fn before_all_tests() {
    SimpleLogger::new().init().unwrap();

    // If this is the first connection to Minecraft we need to reload to activate the minect datapack.
    let mut connection = connection();
    connection
        .execute_commands([Command::new("reload")])
        .unwrap();
    wait_for_connection(&mut connection).await.unwrap();
}

async fn wait_for_connection(
    connection: &mut MinecraftConnection,
) -> Result<Option<SummonNamedEntityOutput>, Elapsed> {
    const INITIAL_CONNECT_ENTITY_NAME: &str = "test_connected";
    let commands = [Command::new(summon_named_entity_command(
        INITIAL_CONNECT_ENTITY_NAME,
    ))];
    let events = connection.add_listener();
    let mut events = events
        .filter_map(|event| event.output.parse::<SummonNamedEntityOutput>().ok())
        .filter(|output| output.name == INITIAL_CONNECT_ENTITY_NAME);
    connection.execute_commands(commands).unwrap();

    const INITIAL_CONNECT_TIMEOUT: Duration = Duration::from_secs(600);
    timeout(INITIAL_CONNECT_TIMEOUT, events.next()).await
}

async fn before_each_test() {
    static BEFORE_ALL_TESTS: OnceCell<()> = OnceCell::const_new();
    BEFORE_ALL_TESTS.get_or_init(before_all_tests).await;
}

const TEST_WORLD_DIR: &str = env!("TEST_WORLD_DIR");
const TEST_LOG_FILE: &str = env!("TEST_LOG_FILE");
const TIMEOUT: Duration = Duration::from_secs(10);

const TEST_DATAPACK_NAME: &str = "mcfd_test";
const DEBUG_DATAPACK_NAME: &str = "mcfd_test_debug";
const TICK_DATAPACK_NAME: &str = "mcfd_tick";

fn get_datapack_dir(name: impl AsRef<Path>) -> std::path::PathBuf {
    Path::new(TEST_WORLD_DIR).join("datapacks").join(name)
}

async fn run_test(
    namespace: &str,
    name: &str,
    after_age_increment: bool,
    debug: bool,
) -> io::Result<()> {
    before_each_test().await;
    // given:
    let test_fn = if debug {
        format!("debug:{}/{}/test", namespace, name)
    } else {
        format!("{}:{}/test", namespace, name)
    };
    create_datapacks(&test_fn, after_age_increment, debug).await?;

    let mut connection = connection();
    let setup_commands = get_setup_commands(after_age_increment, debug);
    if !setup_commands.is_empty() {
        connection.execute_commands(setup_commands)?;
    }

    let mut events = connection.add_named_listener("test");
    let mut breakpoint_events = connection.add_named_listener(LISTENER_NAME);
    let test_commands = get_test_commands(namespace, name, &test_fn, after_age_increment).await;

    // when:
    connection.execute_commands(test_commands)?;

    // then:
    let event = timeout(
        TIMEOUT,
        wait_for_test_output(
            namespace,
            name,
            after_age_increment,
            &mut connection,
            &mut breakpoint_events,
            &mut events,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(event.output, "Summoned new success");

    Ok(())
}

async fn create_datapacks(test_fn: &str, after_age_increment: bool, debug: bool) -> io::Result<()> {
    expand_test_templates().await?;
    if debug {
        create_debug_datapack().await?;
    }
    if after_age_increment {
        create_tick_datapack(&format!("function {}", test_fn)).await?;
    }
    Ok(())
}

async fn expand_test_templates() -> io::Result<()> {
    static START: OnceCell<()> = OnceCell::const_new();
    START.get_or_try_init(do_expand_test_templates).await?;
    Ok(())
}

async fn do_expand_test_templates() -> io::Result<()> {
    include!(concat!(env!("OUT_DIR"), "/tests/expand_test_templates.rs"));

    let test_datapack_dir = get_datapack_dir(TEST_DATAPACK_NAME);
    add_minect_functions(test_datapack_dir).await?;
    Ok(())
}

async fn add_minect_functions(test_datapack_dir: std::path::PathBuf) -> Result<(), io::Error> {
    let connection = connection();
    let minect_datapack_dir = connection.get_datapack_dir();
    if !minect_datapack_dir.is_dir() {
        connection.create_datapack()?;
    }

    let minect_functions = "data/minect/functions";
    let minect_internal_functions = "data/minect_internal/functions";
    try_join!(
        copy_dir(
            minect_datapack_dir.join(minect_functions),
            test_datapack_dir.join(minect_functions),
        ),
        copy_dir(
            minect_datapack_dir.join(minect_internal_functions),
            test_datapack_dir.join(minect_internal_functions),
        ),
    )?;

    Ok(())
}

async fn copy_dir(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    create_dir_all(&dst).await?;
    for entry in WalkDir::new(&src) {
        let entry = entry?;
        let file_type = entry.file_type();
        let relative_path = entry.path().strip_prefix(&src).unwrap();
        let dst_path = dst.as_ref().join(relative_path);
        if file_type.is_dir() && !dst_path.exists() {
            create_dir(dst_path).await?;
        } else if file_type.is_file() {
            copy(entry.path(), dst_path).await?;
        }
    }
    Ok(())
}

async fn create_debug_datapack() -> io::Result<()> {
    static START: OnceCell<()> = OnceCell::const_new();
    START.get_or_try_init(do_create_debug_datapack).await?;
    Ok(())
}

const LISTENER_NAME: &str = "mcfd_test";
const NAMESPACE: &str = "mcfd";

async fn do_create_debug_datapack() -> io::Result<()> {
    let input_path = get_datapack_dir(TEST_DATAPACK_NAME);
    let output_path = get_datapack_dir(DEBUG_DATAPACK_NAME);
    let config = Config {
        namespace: NAMESPACE,
        shadow: false,
        adapter: Some(AdapterConfig {
            adapter_listener_name: LISTENER_NAME,
        }),
    };
    generate_debug_datapack(&input_path, &output_path, &config).await?;
    Ok(())
}

async fn create_tick_datapack(commands: &str) -> io::Result<()> {
    macro_rules! create_tick_template {
        ($path:expr) => {
            create_file(
                get_datapack_dir(TICK_DATAPACK_NAME).join($path),
                include_template!(concat!("mcfd_tick/", $path)),
            )
        };
    }

    try_join!(
        create_tick_template!("data/minecraft/tags/functions/tick.json"),
        create_tick_template!("data/test/functions/tick.mcfunction"),
        create_tick_template!("pack.mcmeta"),
        create_file(
            get_datapack_dir(TICK_DATAPACK_NAME)
                .join("data/test/functions/tick/run_after_age_increment.mcfunction"),
            commands,
        )
    )?;
    Ok(())
}

fn connection() -> MinecraftConnection {
    MinecraftConnection::builder("mcfunction-debugger", TEST_WORLD_DIR)
        .log_file(TEST_LOG_FILE)
        .build()
}

fn get_setup_commands(after_age_increment: bool, debug: bool) -> Vec<Command> {
    let mut commands = Vec::new();

    static SCOREBOARD_ADDED: AtomicBool = AtomicBool::new(false);
    if SCOREBOARD_ADDED.swap(true, Ordering::Relaxed) != true {
        commands.push(Command::new("scoreboard objectives add test_global dummy"));
    }

    let enable_appropriate_datapacks = enable_appropriate_datapacks(after_age_increment, debug);
    if after_age_increment || !enable_appropriate_datapacks.is_empty() {
        // Reload changes to tick datapack and load all datapacks to enable the appropriate ones
        commands.push(Command::new("reload"));
    }
    commands.extend(enable_appropriate_datapacks);

    commands
}

fn enable_appropriate_datapacks(after_age_increment: bool, debug: bool) -> Vec<Command> {
    const UNKNOWN: i8 = -1;
    const FALSE: i8 = 0;
    const TRUE: i8 = 1;
    static DEBUG_DATAPACK_ENABLED: AtomicI8 = AtomicI8::new(UNKNOWN);
    static TEST_DATAPACK_ENABLED: AtomicI8 = AtomicI8::new(UNKNOWN);
    static TICK_DATAPACK_ENABLED: AtomicI8 = AtomicI8::new(UNKNOWN);

    let mut commands = Vec::new();
    if debug {
        if DEBUG_DATAPACK_ENABLED.swap(TRUE, Ordering::Relaxed) != TRUE {
            commands.push(format!(r#"datapack enable "file/{}""#, DEBUG_DATAPACK_NAME));
        }
        if TEST_DATAPACK_ENABLED.swap(TRUE, Ordering::Relaxed) != TRUE {
            commands.push(format!(r#"datapack enable "file/{}""#, TEST_DATAPACK_NAME));
        }
    } else {
        if DEBUG_DATAPACK_ENABLED.swap(FALSE, Ordering::Relaxed) != FALSE {
            commands.push(format!(
                r#"datapack disable "file/{}""#,
                DEBUG_DATAPACK_NAME
            ));
        }
        if TEST_DATAPACK_ENABLED.swap(TRUE, Ordering::Relaxed) != TRUE {
            commands.push(format!(r#"datapack enable "file/{}""#, TEST_DATAPACK_NAME));
        }
    }
    if after_age_increment {
        if debug {
            if TICK_DATAPACK_ENABLED.swap(TRUE, Ordering::Relaxed) != TRUE {
                // Must run before debugger tick.json
                commands.extend([
                    format!(r#"datapack disable "file/{}""#, TICK_DATAPACK_NAME),
                    format!(
                        r#"datapack enable "file/{}" before "file/{}""#,
                        TICK_DATAPACK_NAME, DEBUG_DATAPACK_NAME
                    ),
                ]);
            }
        } else {
            if TICK_DATAPACK_ENABLED.swap(TRUE, Ordering::Relaxed) != TRUE {
                commands.push(format!(r#"datapack enable "file/{}""#, TICK_DATAPACK_NAME));
            }
        }
    } else {
        if TICK_DATAPACK_ENABLED.swap(FALSE, Ordering::Relaxed) != FALSE {
            commands.push(format!(r#"datapack disable "file/{}""#, TICK_DATAPACK_NAME));
        }
    }
    commands.into_iter().map(Command::new).collect()
}

const TRIGGER_TICK_COMMAND: &str = "scoreboard players set tick test_global 1";

async fn get_test_commands(
    namespace: &str,
    name: &str,
    test_fn: &str,
    after_age_increment: bool,
) -> Vec<Command> {
    let mut commands = vec![
        Command::new(running_test_cmd(&test_fn)),
        Command::new(format!("function {}:abort_session", NAMESPACE)),
    ];

    commands.extend(get_breakpoint_commands(namespace, name).await);

    if after_age_increment {
        commands.push(Command::new(TRIGGER_TICK_COMMAND));
    } else {
        commands.push(Command::new(format!("schedule function {} 1", test_fn)));
    }

    commands
}

fn running_test_cmd(test_name: &str) -> String {
    format!("tellraw @a {{\"text\": \"Running test {}\"}}", test_name)
}

async fn get_breakpoint_commands(namespace: &str, name: &str) -> Vec<Command> {
    let mut commands = Vec::new();
    commands.push(Command::new(format!(
        "scoreboard players reset * {}_break",
        NAMESPACE
    )));

    let metadata = read_metadata(get_datapack_dir(DEBUG_DATAPACK_NAME).join("functions.txt")).await;

    let test_fn_dir = get_datapack_dir(TEST_DATAPACK_NAME)
        .join("data")
        .join(namespace)
        .join("functions")
        .join(name);
    for entry in WalkDir::new(&test_fn_dir) {
        let entry = entry.unwrap();
        let file_type = entry.file_type();
        if file_type.is_file() && entry.path().extension() == Some(OsStr::new("mcfunction")) {
            let contents = read_to_string(entry.path()).await.unwrap();
            for (line_index, line) in contents.lines().enumerate() {
                if line.trim() == "# breakpoint" {
                    let path = format!(
                        "{}/{}",
                        name,
                        entry
                            .path()
                            .with_extension("")
                            .strip_prefix(&test_fn_dir)
                            .unwrap()
                            .display()
                    );
                    let fn_name = ResourceLocation::new(namespace, &path);
                    let position = LocalBreakpointPosition {
                        line_number: line_index + 2,
                        position_in_line: BreakpointPositionInLine::Breakpoint,
                    };
                    let score_holder = metadata.get_breakpoint_score_holder(&fn_name, &position);
                    commands.push(Command::new(format!(
                        "scoreboard players set {} {}_break 1",
                        score_holder, NAMESPACE
                    )));
                }
            }
        }
    }
    commands
}

async fn read_metadata(path: impl AsRef<Path>) -> DebugDatapackMetadata {
    let functions = read_to_string(path).await.unwrap();
    let mut fn_ids = HashMap::new();
    for (fn_id, fn_name) in functions.lines().enumerate() {
        let fn_name = ResourceLocationRef::try_from(fn_name).unwrap().to_owned();
        fn_ids.insert(fn_name, fn_id);
    }
    DebugDatapackMetadata::new(fn_ids)
}

async fn wait_for_test_output(
    namespace: &str,
    name: &str,
    after_age_increment: bool,
    connection: &mut MinecraftConnection,
    breakpoint_events: &mut (impl Stream<Item = LogEvent> + Unpin),
    events: &mut (impl Stream<Item = LogEvent> + Unpin),
) -> Option<LogEvent> {
    let event = events.next();
    pin_mut!(event);
    let resume = automatically_resume_breakpoints(
        namespace,
        name,
        after_age_increment,
        connection,
        breakpoint_events,
    );
    pin_mut!(resume);
    match select(event, resume).await {
        Either::Left((result, _)) => result,
        Either::Right(((), _)) => None,
    }
}

async fn automatically_resume_breakpoints(
    namespace: &str,
    name: &str,
    after_age_increment: bool,
    connection: &mut MinecraftConnection,
    events: &mut (impl Stream<Item = LogEvent> + Unpin),
) {
    let mut stopped_events = events
        .filter_map(|event| event.output.parse::<SummonNamedEntityOutput>().ok())
        .filter_map(|output| output.name.parse::<StoppedEvent>().ok());
    while let Some(stopped_event) = stopped_events.next().await {
        on_breakpoint(
            stopped_event,
            namespace,
            name,
            after_age_increment,
            connection,
        )
        .await;
    }
}

async fn on_breakpoint(
    stopped_event: StoppedEvent,
    namespace: &str,
    name: &str,
    after_age_increment: bool,
    connection: &mut MinecraftConnection,
) {
    let mut commands = Vec::from_iter([
        format!("function {}:{}/on_breakpoint", namespace, name),
        format!("function {}:prepare_resume", NAMESPACE),
        format!(
            "function {}:{}/{}/continue_current_iteration_at_{}_{}",
            NAMESPACE,
            stopped_event.position.function.namespace(),
            stopped_event.position.function.path(),
            stopped_event.position.line_number,
            stopped_event.position.position_in_line,
        ),
    ]);
    if !after_age_increment {
        commands = commands
            .into_iter()
            .map(|fn_call| format!("schedule {} 1t", fn_call))
            .collect();
    }
    execute_commands_at_tick_time(connection, commands, after_age_increment).await;
}

async fn execute_commands_at_tick_time(
    connection: &mut MinecraftConnection,
    commands: Vec<String>,
    after_age_increment: bool,
) {
    if after_age_increment {
        create_tick_datapack(&commands.join("\n")).await.unwrap();
        connection
            .execute_commands(Vec::from_iter([
                Command::new("reload"),
                Command::new(TRIGGER_TICK_COMMAND),
            ]))
            .unwrap();
    } else {
        connection
            .execute_commands(commands.into_iter().map(Command::new))
            .unwrap();
    }
}
