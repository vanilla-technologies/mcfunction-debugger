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
    error::{PartialErrorResponse, RequestError},
    DebugAdapterContext,
};
use futures::{
    future::{select, Either},
    pin_mut,
};
use mcfunction_debugger::{
    template_engine::TemplateEngine,
    utils::{logged_command_str, named_logged_command_str},
};
use minect::MinecraftConnection;
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

pub async fn wait_for_connection(
    connection: &mut MinecraftConnection,
    mut context: impl DebugAdapterContext,
) -> Result<(), RequestError<io::Error>> {
    let name = "mcfd_init";
    let mut init_listener = connection.add_listener(name);
    connection
        .inject_commands(Vec::new())
        .map_err(|e| RequestError::Terminate(e))?; // TODO: Hack: connection is not initialized for first inject
    connection
        .inject_commands(vec![
            logged_command_str("function minect:enable_logging"),
            named_logged_command_str(name, "tag @s add mcfd_connection_established"),
            logged_command_str("function minect:reset_logging"),
        ])
        .map_err(|e| RequestError::Terminate(e))?;

    let mut progress_context = context.start_cancellable_progress(
        "Connecting to Minecraft".to_string(),
        Some(
            "If you are connecting for the first time please execute /reload in Minecraft."
                .to_string(),
        ),
    );
    let progress_id = progress_context.progress_id.to_string();

    let init_success = init_listener.recv();
    pin_mut!(init_success);
    let cancel = progress_context.next_cancel_request();
    pin_mut!(cancel);
    match select(init_success, cancel).await {
        Either::Left(_) => {
            let message = Some("Successfully established connection to Minecraft".to_string());
            context.end_cancellable_progress(progress_id, message);
            Ok(())
        }
        Either::Right(_) => {
            let message = Some("Cancelled: Connecting to Minecraft".to_string());
            context.end_cancellable_progress(progress_id, message);

            Err(RequestError::Respond(PartialErrorResponse::new(
                "Successfully cancelled launch.".to_string(),
            )))
        }
    }
}

pub async fn setup_installer_datapack(minecraft_world_dir: impl AsRef<Path>) -> io::Result<()> {
    let minecraft_world_dir = minecraft_world_dir.as_ref();
    let structure_id = read_structure_id(minecraft_world_dir).await;

    let datapacks_dir = minecraft_world_dir.join("datapacks");
    let datapack_dir = datapacks_dir.join("mcfd-installer");
    if datapack_dir.is_dir() {
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
        expand_datapack_template!("data/mcfd_init/functions/choose_chunk.mcfunction"),
        extract_datapack_file!("data/mcfd_init/functions/confirm_chunk.mcfunction"),
        extract_datapack_file!("data/mcfd_init/functions/install.mcfunction"),
        extract_datapack_file!("data/mcfd_init/functions/load.mcfunction"),
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
