use super::*;
use minect::{LoggedCommand, MinecraftConnection, MinecraftConnectionBuilder};
use serial_test::serial;
use std::{
    sync::atomic::{AtomicBool, AtomicI8, Ordering},
    time::Duration,
};
use tokio::{
    sync::OnceCell,
    time::{sleep, timeout},
};

macro_rules! include_template {
    ( $path:expr) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/tests/datapacks/",
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
    let mut expanded = String::with_capacity(string.len());

    let prefix = "say [";
    let mut expanded_until = 0;
    for (start, _) in string.match_indices(prefix) {
        if let Some(end) = string[start..].find("]\n") {
            let end = start + end;
            if let Some((executor, command)) = string[start..end]
                .strip_prefix(prefix)
                .and_then(|it| it.split_once(": "))
            {
                expanded.push_str(&string[expanded_until..start]);
                expanded_until = end + 1;
                expanded.push_str(&log_command(command, executor));
            }
        }
    }
    expanded.push_str(&string[expanded_until..]);
    expanded
}

fn log_command(command: &str, name: &str) -> String {
    LoggedCommand::builder(command.to_string())
        .name(name)
        .build()
        .to_string()
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
        include_test_category!("test_after_age_increment");
    }
}

const TEST_WORLD_DIR: &str = env!("TEST_WORLD_DIR");

async fn run_test(
    namespace: &str,
    name: &str,
    after_age_increment: bool,
    debug: bool,
) -> io::Result<()> {
    // given:
    expand_test_templates().await?;

    let test_fn = if !debug {
        format!("{}:{}/test", namespace, name)
    } else {
        create_debug_datapack().await?;
        format!("debug:{}/{}/test", namespace, name)
    };

    if after_age_increment || debug {
        let on_breakpoint_fn = format!("{}:{}/on_breakpoint", namespace, name);
        create_tick_datapack(&test_fn, &on_breakpoint_fn).await?;
    }

    let mut commands = vec![
        running_test_cmd(&test_fn),
        "function mcfd:clean_up".to_string(),
    ];
    enable_appropriate_datapacks(&mut commands, after_age_increment, debug);
    if after_age_increment {
        commands.push("scoreboard players set tick test_global 1".to_string());
    } else {
        commands.push(format!("schedule function {} 1", test_fn));
    }

    let mut connection = connection();
    let mut events = connection.add_listener("test");

    // when:
    connection.inject_commands(commands)?;

    // then:
    let event = timeout(TIMEOUT, events.recv()).await?.unwrap();
    assert_eq!(event.message, "Added tag 'success' to test");

    Ok(())
}

fn enable_appropriate_datapacks(
    commands: &mut Vec<String>,
    after_age_increment: bool,
    debug: bool,
) {
    const UNKNOWN: i8 = -1;
    const FALSE: i8 = 0;
    const TRUE: i8 = 1;
    static DEBUG_DATAPACK_ENABLED: AtomicI8 = AtomicI8::new(UNKNOWN);
    static TICK_DATAPACK_INITIALIZED: AtomicBool = AtomicBool::new(false);

    if debug {
        if DEBUG_DATAPACK_ENABLED.swap(TRUE, Ordering::SeqCst) != TRUE {
            commands.push(r#"datapack enable "file/mcfd_test_debug""#.to_string());
        }
        if after_age_increment {
            if TICK_DATAPACK_INITIALIZED.swap(true, Ordering::SeqCst) != true {
                // Must run before debugger tick.json
                commands.extend([
                    r#"datapack disable "file/mcfd_tick""#.to_string(),
                    r#"datapack enable "file/mcfd_tick" before "file/mcfd_test_debug""#.to_string(),
                ]);
            }
        }
    } else {
        if DEBUG_DATAPACK_ENABLED.swap(FALSE, Ordering::SeqCst) != FALSE {
            commands.push(r#"datapack disable "file/mcfd_test_debug""#.to_string());
        }
    }
}

async fn expand_test_templates() -> io::Result<()> {
    static START: OnceCell<()> = OnceCell::const_new();
    START.get_or_try_init(do_expand_test_templates).await?;
    Ok(())
}

async fn do_expand_test_templates() -> io::Result<()> {
    include!(concat!(env!("OUT_DIR"), "/tests/expand_test_templates.rs"));
    wait_for_mount().await;
    Ok(())
}

fn connection() -> MinecraftConnection {
    MinecraftConnectionBuilder::from_ref("test", TEST_WORLD_DIR).build()
}

fn running_test_cmd(test_name: &str) -> String {
    format!("tellraw @a {{\"text\": \"Running test {}\"}}", test_name)
}

async fn create_debug_datapack() -> io::Result<()> {
    static START: OnceCell<()> = OnceCell::const_new();
    START.get_or_try_init(do_create_debug_datapack).await?;
    Ok(())
}

async fn do_create_debug_datapack() -> io::Result<()> {
    let input_path = Path::new(TEST_WORLD_DIR).join("datapacks/mcfd_test");
    let output_path = Path::new(TEST_WORLD_DIR).join("datapacks/mcfd_test_debug");
    generate_debug_datapack(&input_path, &output_path, "mcfd", false).await?;
    wait_for_mount().await;
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
    wait_for_mount().await;
    Ok(())
}

async fn wait_for_mount() {
    sleep(Duration::from_secs(1)).await;
}

const TIMEOUT: Duration = Duration::from_secs(10);
