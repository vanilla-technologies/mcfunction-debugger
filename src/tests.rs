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
        if let Some(end) = string[start..].find(']') {
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
    ($namespace:ident, $name:ident, $path:literal $(, $paths:literal)*) => {
        paste! {
            #[tokio::test]
            #[serial]
            async fn [<$namespace _ $name _minecraft>]() -> io::Result<()> {
                // given:
                let mut connection = connection();

                let commands = vec![
                    running_test_cmd(concat!(stringify!($namespace), ":", stringify!($name), "_minecraft")),
                    concat!("function ", stringify!($namespace), ":", stringify!($name), "/test").to_string(),
                ];

                expand_test_templates!("mcfd_test/pack.mcmeta", $path $(, $paths)*);

                wait_for_mount().await;

                let mut events = connection.add_listener("test");

                // when:
                connection.inject_commands(commands)?;

                // then:
                let event = timeout(TIMEOUT, events.recv())
                    .await?
                    .unwrap();
                assert_eq!(event.message, "Added tag 'success' to test");

                Ok(())
            }

            #[tokio::test]
            #[serial]
            async fn [<$namespace _ $name _debug>]() -> io::Result<()> {
                // given:
                let mut connection = connection();

                let commands = vec![
                    running_test_cmd(concat!(stringify!($namespace), ":", stringify!($name), "_minecraft")),
                    concat!("function debug:", stringify!($namespace), "/", stringify!($name), "/test").to_string(),
                ];

                expand_test_templates!("mcfd_test/pack.mcmeta", $path $(, $paths)*);

                let test_datapack_path = Path::new(TEST_WORLD_DIR).join("datapacks/mcfd_test/");
                let output_path = Path::new(TEST_WORLD_DIR).join("datapacks/mcfd_test_debug/");
                generate_debug_datapack(&test_datapack_path, "mcfd", &output_path, false).await?;

                wait_for_mount().await;

                let mut events = connection.add_listener("test");

                // when:
                connection.inject_commands(commands)?;

                // then:
                let event = timeout(TIMEOUT, events.recv())
                    .await?
                    .unwrap();
                assert_eq!(event.message, "Added tag 'success' to test");

                Ok(())
            }
        }
    };
}

macro_rules! test_after_age_increment {
    ($namespace:ident, $name:ident, $path:literal $(, $paths:literal)*) => {
        paste! {
            #[tokio::test]
            #[serial]
            async fn [<test_after_age_increment_ $name _minecraft>]() -> io::Result<()> {
                // given:
                let mut connection = connection();

                let commands = vec![
                    running_test_cmd(concat!(stringify!($namespace), ":", stringify!($name), "_minecraft")),
                    "scoreboard players set tick test_global 1".to_string(),
                ];

                expand_test_templates!("mcfd_test/pack.mcmeta", $path $(, $paths)*);

                create_tick_datapack(concat!(stringify!($namespace), ":", stringify!($name), "/test")).await?;

                wait_for_mount().await;

                let mut events = connection.add_listener("test");

                // when:
                connection.inject_commands(commands)?;

                // then:
                let event = timeout(TIMEOUT, events.recv())
                    .await?
                    .unwrap();
                assert_eq!(event.message, "Added tag 'success' to test");

                Ok(())
            }

            #[tokio::test]
            #[serial]
            async fn [<test_after_age_increment_ $name _debug>]() -> io::Result<()> {
                // given:
                let mut connection = connection();

                let commands = vec![
                    running_test_cmd(concat!(stringify!($namespace), ":", stringify!($name), "_debug")),
                    "scoreboard players set tick test_global 1".to_string(),
                    // Must run before debugger tick.json
                    r#"datapack disable "file/mcfd_tick""#.to_string(),
                    r#"datapack enable "file/mcfd_tick" first"#.to_string(),
                ];

                expand_test_templates!("mcfd_test/pack.mcmeta", $path $(, $paths)*);

                let test_datapack_path = Path::new(TEST_WORLD_DIR).join("datapacks/mcfd_test");
                let output_path = Path::new(TEST_WORLD_DIR).join("datapacks/mcfd_test_debug");
                generate_debug_datapack(&test_datapack_path, "mcfd", &output_path, false).await?;

                create_tick_datapack(concat!("debug:", stringify!($namespace), "/", stringify!($name), "/test")).await?;

                wait_for_mount().await;

                let mut events = connection.add_listener("test");

                // when:
                connection.inject_commands(commands)?;

                // then:
                let event = timeout(TIMEOUT, events.recv())
                    .await?
                    .unwrap();
                assert_eq!(event.message, "Added tag 'success' to test");

                Ok(())
            }
        }
    };
}

macro_rules! test {
    ($namespace:ident, $name:ident, $path:literal $(, $paths:literal)*) => {
        test_before_age_increment!($namespace, $name, $path $(, $paths)*);
    }
}

include!(concat!(env!("OUT_DIR"), "/tests.rs"));

// Additionally run schedule_gametime_1t after age increment to ensure that all tests are run
// before the tick.json of the debugger, just as if they were executed by a user.
test_after_age_increment!(
    test,
    schedule_gametime_1t,
    "mcfd_test/data/test/functions/schedule_gametime_1t/test.mcfunction",
    "mcfd_test/data/test/functions/schedule_gametime_1t/scheduled.mcfunction"
);

const TEST_WORLD_DIR: &str = env!("TEST_WORLD_DIR");

fn connection() -> MinecraftConnection {
    MinecraftConnectionBuilder::from_ref("test", TEST_WORLD_DIR).build()
}

fn running_test_cmd(test_name: &str) -> String {
    format!("tellraw @a {{\"text\": \"Running test {}\"}}", test_name)
}

async fn create_tick_datapack(function: &str) -> io::Result<()> {
    macro_rules! create_tick_template {
        ($path:expr) => {
            expand_template!(concat!("mcfd_tick/", $path), |raw: &str| {
                raw.replace("-fn-", function)
            })
        };
    }
    create_tick_template!("pack.mcmeta")?;
    create_tick_template!("data/minecraft/tags/functions/tick.json")?;
    create_tick_template!("data/test/functions/-test_name-/tick.mcfunction")?;
    Ok(())
}

async fn wait_for_mount() {
    sleep(Duration::from_secs(1)).await;
}

const TIMEOUT: Duration = Duration::from_secs(5);