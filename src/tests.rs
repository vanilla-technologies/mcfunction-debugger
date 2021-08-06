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

macro_rules! expand_test_template {
    ($path:expr) => {
        expand_template!($path, expand_logged_cmds)
    };
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

                let commands = to_test_commands(
                    concat!(stringify!($namespace), ":", stringify!($name), "_minecraft"),
                    include_template!($path),
                );

                expand_test_templates!("mcfd_test/pack.mcmeta" $(, $paths)*);

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

                let commands = to_test_commands(
                    concat!(stringify!($namespace), ":", stringify!($name), "_debug"),
                    concat!("function debug:", stringify!($namespace), "/", stringify!($name), "/test"),
                );

                expand_test_templates!("mcfd_test/pack.mcmeta", $path $(, $paths)*);

                let test_datapack_path = Path::new(TEST_WORLD_DIR).join("datapacks/mcfd_test/");
                let output_path = Path::new(TEST_WORLD_DIR).join("datapacks/mcfd_test_debug/");
                let namespace = "mcfd";

                generate_debug_datapack(&test_datapack_path, namespace, &output_path, false).await?;

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

const TEST_WORLD_DIR: &str = env!("TEST_WORLD_DIR");

fn connection() -> MinecraftConnection {
    MinecraftConnectionBuilder::from_ref("test", TEST_WORLD_DIR).build()
}

fn to_test_commands(test_name: &str, function_contents: &str) -> Vec<String> {
    let mut commands = vec![running_test_cmd(test_name)];
    commands.extend(to_commands(function_contents));
    commands
}

fn running_test_cmd(test_name: &str) -> String {
    format!("tellraw @a {{\"text\": \"Running test {}\"}}", test_name)
}

fn to_commands(function_contents: &str) -> impl Iterator<Item = String> + '_ {
    function_contents
        .split_terminator('\n')
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .map(|it| it.to_string())
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

async fn create_file(path: impl AsRef<Path>, contents: &str) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        create_dir_all(parent).await?;
    }
    write(path, contents).await
}

const TIMEOUT: Duration = Duration::from_secs(5);

async fn wait_for_mount() {
    sleep(Duration::from_secs(1)).await;
}
