// mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any
// Minecraft mods.
//
// © Copyright (C) 2021 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
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

use crate::{
    adapter::inject_commands,
    error::{PartialErrorResponse, RequestError},
    minecraft::parse_added_tag_message,
    DebugAdapterContext,
};
use futures::{
    future::{select, Either},
    pin_mut,
};
use mcfunction_debugger::{
    template_engine::TemplateEngine,
    utils::{logged_command_str, named_logged_command},
};
use minect::{log_observer::LogEvent, MinecraftConnection};
use std::{collections::BTreeMap, io, iter::FromIterator, path::Path};
use tokio::{
    fs::{create_dir_all, read_to_string, remove_dir_all, write},
    try_join,
};

macro_rules! include_template {
    ($path:expr) => {
        include_str!(concat!("installer_datapack/", $path))
    };
}

macro_rules! extract_file {
    ($out_path:expr, $relative_path:literal) => {
        create_file(
            $out_path.join($relative_path),
            include_template!($relative_path),
        )
    };
}

macro_rules! expand_template {
    ($engine:expr, $out_path:expr, $relative_path:expr) => {{
        let path = $out_path.join($engine.expand($relative_path));
        let content = $engine.expand(include_template!($relative_path));
        create_file(path, content)
    }};
}

pub async fn setup_installer_datapack(minecraft_world_dir: impl AsRef<Path>) -> io::Result<()> {
    let minecraft_world_dir = minecraft_world_dir.as_ref();
    let structure_id = read_structure_id(minecraft_world_dir).await;

    let datapack_dir = get_installer_datapack_dir(minecraft_world_dir);
    if datapack_dir.as_ref().is_dir() {
        remove_dir_all(&datapack_dir).await?;
    }
    extract_installer_datapack(&datapack_dir, structure_id).await?;
    Ok(())
}

async fn read_structure_id(minecraft_world_dir: &Path) -> u64 {
    let id_txt_path = minecraft_world_dir.join("generated/minect/structures/dap/id.txt");
    let content = read_to_string(id_txt_path).await;
    if let Ok(content) = content {
        if !content.is_empty() {
            let id = content.parse::<u64>();
            if let Ok(id) = id {
                id.wrapping_add(1)
            } else {
                0
            }
        } else {
            0
        }
    } else {
        0
    }
}

fn get_installer_datapack_dir(minecraft_world_dir: impl AsRef<Path>) -> impl AsRef<Path> {
    minecraft_world_dir
        .as_ref()
        .join("datapacks/mcfd-installer")
}

async fn extract_installer_datapack(
    datapack_dir: impl AsRef<Path>,
    structure_id: u64,
) -> io::Result<()> {
    let datapack_dir = datapack_dir.as_ref();
    macro_rules! extract_datapack_file {
        ($relative_path:literal) => {
            extract_file!(datapack_dir, $relative_path)
        };
    }
    let structure_id = structure_id.to_string();
    let engine = TemplateEngine::new(
        BTreeMap::from_iter([("-structure_id-", structure_id.as_str())]),
        None,
    );
    macro_rules! expand_datapack_template {
        ($relative_path:literal) => {
            expand_template!(engine, datapack_dir, $relative_path)
        };
    }
    try_join!(
        extract_datapack_file!("data/mcfd_init/functions/cancel_cleanup.mcfunction"),
        extract_datapack_file!("data/mcfd_init/functions/cancel.mcfunction"),
        expand_datapack_template!("data/mcfd_init/functions/choose_chunk.mcfunction"),
        extract_datapack_file!("data/mcfd_init/functions/confirm_chunk.mcfunction"),
        extract_datapack_file!("data/mcfd_init/functions/install.mcfunction"),
        extract_datapack_file!("data/mcfd_init/functions/load.mcfunction"),
        extract_datapack_file!("data/mcfd_init/functions/reload.mcfunction"),
        extract_datapack_file!("data/mcfd_init/functions/remove_chunk_choice.mcfunction"),
        extract_datapack_file!("data/mcfd_init/functions/uninstall.mcfunction"),
        extract_datapack_file!("data/minecraft/tags/functions/load.json"),
        extract_datapack_file!("pack.mcmeta"),
    )?;
    Ok(())
}

async fn create_file(path: impl AsRef<Path>, contents: impl AsRef<str>) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        create_dir_all(parent).await?;
    }
    write(path, contents.as_ref()).await
}

const SUCCESS_TAG: &str = "mcfd_init_success";

pub async fn wait_for_connection(
    connection: &mut MinecraftConnection,
    minecraft_world_dir: impl AsRef<Path>,
    mut context: impl DebugAdapterContext,
) -> Result<(), RequestError<io::Error>> {
    const LISTENER_NAME: &str = "mcfd_init"; // Hardcoded in installer datapack as well
    let mut init_listener = connection.add_listener(LISTENER_NAME);
    inject_commands(connection, Vec::new()).map_err(|e| RequestError::Terminate(e))?; // TODO: Hack: connection is not initialized for first inject
    inject_commands(
        connection,
        vec![
            logged_command_str("function minect:enable_logging"),
            named_logged_command(LISTENER_NAME, format!("tag @s add {}", SUCCESS_TAG)),
            logged_command_str("function minect:reset_logging"),
        ],
    )
    .map_err(|e| RequestError::Terminate(e))?;

    let mut progress_context = context.start_cancellable_progress(
        "Connecting to Minecraft".to_string(),
        Some(
            "If you are connecting for the first time please execute /reload in Minecraft."
                .to_string(),
        ),
    );
    let progress_id = progress_context.progress_id.to_string();

    let init_result = init_listener.recv();
    pin_mut!(init_result);
    let cancel = progress_context.next_cancel_request();
    pin_mut!(cancel);
    let success = match select(init_result, cancel).await {
        Either::Left((log_event, _)) => is_install_success(log_event),
        Either::Right(_) => false,
    };

    let message = Some(if success {
        "Successfully established connection to Minecraft".to_string()
    } else {
        "Cancelled: Connecting to Minecraft".to_string()
    });
    context.end_cancellable_progress(progress_id, message);

    let datapack_dir = get_installer_datapack_dir(minecraft_world_dir);
    if datapack_dir.as_ref().is_dir() {
        remove_dir_all(&datapack_dir)
            .await
            .map_err(RequestError::Terminate)?;
    }

    if success {
        Ok(())
    } else {
        Err(RequestError::Respond(PartialErrorResponse::new(
            "Launch was cancelled.".to_string(),
        )))
    }
}

fn is_install_success(log_event: Option<LogEvent>) -> bool {
    if let Some(log_event) = log_event {
        if let Some(tag) = parse_added_tag_message(&log_event.message) {
            if tag == SUCCESS_TAG {
                return true;
            }
        }
    }
    false
}
