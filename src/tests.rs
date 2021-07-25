use super::*;
use minect::{LoggedCommand, MinecraftConnection, MinecraftConnectionBuilder};
use serial_test::serial;
use std::time::Duration;
use tokio::time::{sleep, timeout};

macro_rules! create_function {
    ($path:expr) => {
        create_file(
            Path::new(TEST_WORLD_DIR).join(concat!("datapacks/mcfd_test/", $path)),
            &expand_function(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test/datapack_template/",
                $path
            ))),
        )
        .await
    };
}

macro_rules! create_functions {
    () => {};
    ($path:expr $(, $paths:expr)*) => {{
        create_function!($path)?;
        create_functions!($($paths),*);
    }};
}

macro_rules! test {
    ($name:ident, $fn_name_debug:ident, $path:literal $(, $paths:literal),*) => {
        #[tokio::test]
        #[serial]
        async fn $name() -> io::Result<()> {
            // given:
            let mut connection = connection();

            let commands = to_commands(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test/datapack_template/",
                $path
            )));

            create_functions!("pack.mcmeta", $($paths),*);

            sleep(Duration::from_millis(500)).await; // Wait for mount

            let mut events = connection.add_listener("test");

            // when:
            connection.inject_commands(commands)?;

            // then:
            let event = timeout(Duration::from_secs(5), events.recv())
                .await?
                .unwrap();
            assert_eq!(event.message, "Added tag 'success' to test");

            Ok(())
        }

        #[tokio::test]
        #[serial]
        async fn $fn_name_debug() -> io::Result<()> {
            // given:
            let mut connection = connection();

            let commands = to_commands(concat!("function debug:test/", stringify!($name), "/test"));

            create_functions!("pack.mcmeta", $path, $($paths),*);

            let test_datapack_path = Path::new(TEST_WORLD_DIR).join("datapacks/mcfd_test/");
            let output_path = Path::new(TEST_WORLD_DIR).join("datapacks/mcfd_test_debug/");
            let namespace = "mcfd";

            generate_debug_datapack(&test_datapack_path, namespace, &output_path, false).await?;

            sleep(Duration::from_millis(500)).await; // Wait for mount

            let mut events = connection.add_listener("test");

            // when:
            connection.inject_commands(commands)?;

            // then:
            let event = timeout(Duration::from_secs(5), events.recv())
                .await?
                .unwrap();
            assert_eq!(event.message, "Added tag 'success' to test");

            Ok(())
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/tests.rs"));

const TEST_WORLD_DIR: &str = env!("TEST_WORLD_DIR");

fn connection() -> MinecraftConnection {
    MinecraftConnectionBuilder::from_ref("test", TEST_WORLD_DIR).build()
}

fn to_commands(function_contents: &str) -> Vec<String> {
    function_contents
        .split_terminator('\n')
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .map(|it| it.to_string())
        .collect()
}

fn expand_function(string: &str) -> String {
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
