use super::*;
use minect::{LoggedCommand, MinecraftConnection, MinecraftConnectionBuilder};
use paste::paste;
use serial_test::serial;
use std::time::Duration;
use tokio::time::{sleep, timeout};

macro_rules! include_template {
    ( $path:expr) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test/datapack_templates/",
            $path
        ))
    };
}

macro_rules! expand_template {
    ($path:expr, $expand:expr) => {{
        let expand = $expand;
        create_file(
            Path::new(TEST_WORLD_DIR)
                .join("datapacks")
                .join(expand($path)),
            &expand(include_template!($path)),
        )
        .await
    }};
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

macro_rules! expand_test_templates {
    () => {};
    ($path:expr $(, $paths:expr)*) => {{
        expand_test_template!($path)?;
        expand_test_templates!($($paths),*);
    }};
}

macro_rules! test_before_age_increment {
    ($namespace:ident, $name:ident, $($paths:expr),+) => {
        paste! {
            #[tokio::test]
            #[serial]
            async fn [<test_before_age_increment_ $name _minecraft>]() -> io::Result<()> {
                expand_test_templates!("mcfd_test/pack.mcmeta", $($paths),+);
                run_test(stringify!($namespace), stringify!($name), false, false).await
            }

            #[tokio::test]
            #[serial]
            async fn [<test_before_age_increment_ $name _debug>]() -> io::Result<()> {
                expand_test_templates!("mcfd_test/pack.mcmeta", $($paths),+);
                run_test(stringify!($namespace), stringify!($name), false, true).await
            }
        }
    };
}

macro_rules! test_after_age_increment {
    ($namespace:ident, $name:ident, $($paths:expr),+) => {
        paste! {
            #[tokio::test]
            #[serial]
            async fn [<test_after_age_increment_ $name _minecraft>]() -> io::Result<()> {
                expand_test_templates!("mcfd_test/pack.mcmeta", $($paths),+);
                run_test(stringify!($namespace), stringify!($name), true, false).await
            }

            #[tokio::test]
            #[serial]
            async fn [<test_after_age_increment_ $name _debug>]() -> io::Result<()> {
                expand_test_templates!("mcfd_test/pack.mcmeta", $($paths),+);
                run_test(stringify!($namespace), stringify!($name), true, true).await
            }
        }
    };
}

macro_rules! test {
    ($namespace:ident, $name:ident, $($paths:expr),+) => {
        test_before_age_increment!($namespace, $name, $($paths),+);
        test_after_age_increment!($namespace, $name, $($paths),+);
    }
}

include!(concat!(env!("OUT_DIR"), "/tests.rs"));

const TEST_WORLD_DIR: &str = env!("TEST_WORLD_DIR");

async fn run_test(
    namespace: &str,
    name: &str,
    after_age_increment: bool,
    debug: bool,
) -> io::Result<()> {
    // given:
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

    let mut commands = vec![running_test_cmd(&test_fn)];
    if debug {
        commands.push(r#"datapack enable "file/mcfd_test_debug""#.to_string());
        if after_age_increment {
            // Must run before debugger tick.json
            commands.extend([
                r#"datapack disable "file/mcfd_tick""#.to_string(),
                r#"datapack enable "file/mcfd_tick" before "file/mcfd_test_debug""#.to_string(),
            ]);
        }
    } else {
        commands.push(r#"datapack disable "file/mcfd_test_debug""#.to_string());
    }
    if after_age_increment {
        commands.push("scoreboard players set tick test_global 1".to_string());
    } else {
        commands.push(format!("schedule function {} 1", test_fn));
    }

    wait_for_mount().await;

    let mut connection = connection();
    let mut events = connection.add_listener("test");

    // when:
    connection.inject_commands(commands)?;

    // then:
    let event = timeout(TIMEOUT, events.recv()).await?.unwrap();
    assert_eq!(event.message, "Added tag 'success' to test");

    Ok(())
}

fn connection() -> MinecraftConnection {
    MinecraftConnectionBuilder::from_ref("test", TEST_WORLD_DIR).build()
}

fn running_test_cmd(test_name: &str) -> String {
    format!("tellraw @a {{\"text\": \"Running test {}\"}}", test_name)
}

async fn create_debug_datapack() -> io::Result<()> {
    let input_path = Path::new(TEST_WORLD_DIR).join("datapacks/mcfd_test");
    let output_path = Path::new(TEST_WORLD_DIR).join("datapacks/mcfd_test_debug");
    generate_debug_datapack(&input_path, "mcfd", &output_path, false).await
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
    create_tick_template!("pack.mcmeta")?;
    create_tick_template!("data/minecraft/tags/functions/tick.json")?;
    create_tick_template!("data/test/functions/tick.mcfunction")?;
    Ok(())
}

async fn wait_for_mount() {
    sleep(Duration::from_secs(1)).await;
}

const TIMEOUT: Duration = Duration::from_secs(10);
