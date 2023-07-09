use mcfunction_debugger::generator::{config::Config, generate_debug_datapack};
use minect::{
    command::{named_logged_block_commands, summon_named_entity_command, SummonNamedEntityOutput},
    Command, MinecraftConnection,
};
use serial_test::serial;
use simple_logger::SimpleLogger;
use std::{
    io,
    path::Path,
    sync::atomic::{AtomicBool, AtomicI8, Ordering},
    time::Duration,
};
use tokio::{
    fs::{copy, create_dir, create_dir_all, write},
    sync::OnceCell,
    time::{error::Elapsed, timeout},
    try_join,
};
use tokio_stream::StreamExt;
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
    create_datapacks(namespace, name, &test_fn, after_age_increment, debug).await?;

    let mut connection = connection();
    let setup_commands = get_setup_commands(after_age_increment, debug);
    if !setup_commands.is_empty() {
        connection.execute_commands(setup_commands)?;
    }

    let mut events = connection.add_named_listener("test");
    let test_commands = get_test_commands(&test_fn, after_age_increment);

    // when:
    connection.execute_commands(test_commands)?;

    // then:
    let event = timeout(TIMEOUT, events.next()).await?.unwrap();
    assert_eq!(event.output, "Summoned new success");

    Ok(())
}

async fn create_datapacks(
    namespace: &str,
    name: &str,
    test_fn: &str,
    after_age_increment: bool,
    debug: bool,
) -> io::Result<()> {
    expand_test_templates().await?;
    if debug {
        create_debug_datapack().await?;
    }
    Ok(if after_age_increment || debug {
        let on_breakpoint_fn = format!("{}:{}/on_breakpoint", namespace, name);
        create_tick_datapack(test_fn, &on_breakpoint_fn).await?;
    })
}

async fn expand_test_templates() -> io::Result<()> {
    static START: OnceCell<()> = OnceCell::const_new();
    START.get_or_try_init(do_expand_test_templates).await?;
    Ok(())
}

async fn do_expand_test_templates() -> io::Result<()> {
    include!(concat!(env!("OUT_DIR"), "/tests/expand_test_templates.rs"));

    let test_datapack_dir = Path::new(TEST_WORLD_DIR).join("datapacks/mcfd_test");
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

async fn do_create_debug_datapack() -> io::Result<()> {
    let input_path = Path::new(TEST_WORLD_DIR).join("datapacks/mcfd_test");
    let output_path = Path::new(TEST_WORLD_DIR).join("datapacks/mcfd_test_debug");
    let config = Config {
        namespace: "mcfd",
        shadow: false,
        adapter: None,
    };
    generate_debug_datapack(&input_path, &output_path, &config).await?;
    Ok(())
}

async fn create_tick_datapack(test_fn: &str, on_breakpoint_fn: &str) -> io::Result<()> {
    macro_rules! create_tick_template {
        ($path:expr) => {
            expand_template!(concat!("mcfd_tick/", $path), |raw: &str| {
                raw.replace("-test-", test_fn)
                    .replace("-on_breakpoint-", on_breakpoint_fn)
            })
        };
    }
    try_join!(
        create_tick_template!("data/minecraft/tags/functions/tick.json"),
        create_tick_template!("data/test/functions/tick.mcfunction"),
        create_tick_template!("data/test/functions/tick/on_breakpoint.mcfunction"),
        create_tick_template!("pack.mcmeta"),
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
            commands.push(Command::new(r#"datapack enable "file/mcfd_test_debug""#));
        }
        if TEST_DATAPACK_ENABLED.swap(TRUE, Ordering::Relaxed) != TRUE {
            commands.push(Command::new(r#"datapack enable "file/mcfd_test""#));
        }
        if TICK_DATAPACK_ENABLED.swap(TRUE, Ordering::Relaxed) != TRUE {
            // Must run before debugger tick.json
            commands.extend([
                Command::new(r#"datapack disable "file/mcfd_tick""#),
                Command::new(r#"datapack enable "file/mcfd_tick" before "file/mcfd_test_debug""#),
            ]);
        }
    } else {
        if DEBUG_DATAPACK_ENABLED.swap(FALSE, Ordering::Relaxed) != FALSE {
            commands.push(Command::new(r#"datapack disable "file/mcfd_test_debug""#));
        }
        if TEST_DATAPACK_ENABLED.swap(TRUE, Ordering::Relaxed) != TRUE {
            commands.push(Command::new(r#"datapack enable "file/mcfd_test""#));
        }
        if after_age_increment {
            if TICK_DATAPACK_ENABLED.swap(TRUE, Ordering::Relaxed) != TRUE {
                commands.push(Command::new(r#"datapack enable "file/mcfd_tick""#));
            }
        } else {
            if TICK_DATAPACK_ENABLED.swap(FALSE, Ordering::Relaxed) != FALSE {
                commands.push(Command::new(r#"datapack disable "file/mcfd_tick""#));
            }
        }
    }
    commands
}

fn get_test_commands(test_fn: &str, after_age_increment: bool) -> Vec<Command> {
    let mut commands = vec![
        Command::new(running_test_cmd(&test_fn)),
        Command::new("function mcfd:clean_up"),
    ];

    if after_age_increment {
        commands.push(Command::new("scoreboard players set tick test_global 1"));
    } else {
        commands.push(Command::new(format!("schedule function {} 1", test_fn)));
    }

    commands
}

fn running_test_cmd(test_name: &str) -> String {
    format!("tellraw @a {{\"text\": \"Running test {}\"}}", test_name)
}
