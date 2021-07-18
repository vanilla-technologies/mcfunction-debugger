use super::*;
use minect::{Command, CommandBuilder, InjectionConnection};
use std::time::Duration;
use tokio::time::{sleep, timeout};

macro_rules! create_function {
    ($path:literal) => {
        create_file(
            Path::new(TEST_WORLD_DIR).join(concat!("datapacks/minect/", $path)),
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
    ($path:literal $(, $paths:literal),*) => {{
        create_function!($path)?;
        create_functions!($($paths:literal),*);
    }};
}

macro_rules! test {
    ($name:ident $(, $paths:literal),*) => {
        #[tokio::test]
        async fn $name() -> io::Result<()> {
            // given:
            let mut connection = connection();

            let group = to_group(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test/datapack_template/data/test/functions/",
                stringify!($name),
                "/test.mcfunction"
            )));

            create_functions!($($paths),*);

            sleep(Duration::from_millis(500)).await; // Wait for mount

            let mut events = connection.add_listener("test");

            // when:
            connection.inject_group(group)?;

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

fn connection() -> InjectionConnection {
    let panic_invalid_dir = || {
        panic!(
            "Expected world_dir to be in .minecraft/saves, but was: {}",
            TEST_WORLD_DIR
        )
    };
    let world_dir = Path::new(TEST_WORLD_DIR);
    let minecraft_dir = world_dir
        .parent()
        .unwrap_or_else(panic_invalid_dir)
        .parent()
        .unwrap_or_else(panic_invalid_dir);
    let log_file = minecraft_dir.join("logs/latest.log");
    InjectionConnection::new("test", &world_dir, log_file)
}

fn to_group(function_contents: &str) -> Vec<Command> {
    let group = function_contents
        .split_terminator('\n')
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .map(|it| CommandBuilder::new(it).build())
        .collect::<Vec<_>>();
    group
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
    CommandBuilder::new(command)
        .name(Some(name))
        .build()
        .to_string()
}

async fn create_file(path: impl AsRef<Path>, contents: &str) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        create_dir_all(parent).await?;
    }
    write(path, contents).await
}
