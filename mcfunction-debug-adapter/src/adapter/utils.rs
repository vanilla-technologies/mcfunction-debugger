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
    adapter::{MinecraftSession, ADAPTER_LISTENER_NAME},
    error::{DapError, PartialErrorResponse},
};
use futures::{Stream, StreamExt};
use mcfunction_debugger::{
    generate_debug_datapack,
    parser::command::resource_location::{ResourceLocation, ResourceLocationRef},
    AdapterConfig, Config, LocalBreakpoint,
};
use minect::log_observer::LogEvent;
use multimap::MultiMap;
use std::{
    fmt::Display,
    future::ready,
    path::{Path, PathBuf},
    str::FromStr,
};
use tokio::fs::remove_dir_all;
use tokio_stream::wrappers::UnboundedReceiverStream;

pub fn parse_function_path(path: impl AsRef<Path>) -> Result<(PathBuf, ResourceLocation), String> {
    let datapack = find_parent_datapack(path.as_ref()).ok_or_else(|| {
        format!(
            "does not denote a path in a datapack directory with a pack.mcmeta file: {}",
            &path.as_ref().display()
        )
    })?;
    let data_path = path
        .as_ref()
        .strip_prefix(datapack.join("data"))
        .map_err(|_| {
            format!(
                "does not denote a path in the data directory of datapack {}: {}",
                &datapack.display(),
                &path.as_ref().display()
            )
        })?;
    let function = get_function_name(data_path, &path)?;
    Ok((datapack.to_path_buf(), function))
}

pub fn find_parent_datapack(mut path: &Path) -> Option<&Path> {
    while let Some(p) = path.parent() {
        path = p;
        let pack_mcmeta_path = path.join("pack.mcmeta");
        if pack_mcmeta_path.is_file() {
            return Some(path);
        }
    }
    None
}

pub fn get_function_name(
    data_path: impl AsRef<Path>,
    path: impl AsRef<Path>,
) -> Result<ResourceLocation, String> {
    let namespace = data_path.as_ref()
        .iter()
        .next()
        .ok_or_else(|| {
            format!(
                "contains an invalid path: {}",
                path.as_ref().display()
            )
        })?
        .to_str()
        .unwrap() // Path is known to be UTF-8
        ;
    let fn_path = data_path
        .as_ref()
        .strip_prefix(Path::new(namespace).join("functions"))
        .map_err(|_| format!("contains an invalid path: {}", path.as_ref().display()))?
        .with_extension("")
        .to_str()
        .unwrap() // Path is known to be UTF-8
        .replace(std::path::MAIN_SEPARATOR, "/");
    Ok(ResourceLocation::new(&namespace, &fn_path))
}

pub(super) async fn generate_datapack(
    minecraft_session: &MinecraftSession,
    breakpoints: &MultiMap<ResourceLocation, LocalBreakpoint>,
    generated_breakpoints: &MultiMap<ResourceLocation, LocalBreakpoint>,
) -> Result<(), DapError> {
    let mut breakpoints = breakpoints.clone();

    // Add all generated breakpoints that are not at the same position as user breakpoints
    for (key, values) in generated_breakpoints.iter_all() {
        for value in values {
            if !contains_breakpoint(
                &breakpoints,
                &McfunctionBreakpoint {
                    function: key.clone(),
                    line_number: value.line_number,
                },
            ) {
                breakpoints.insert(key.clone(), value.clone());
            }
        }
    }

    let config = Config {
        namespace: &minecraft_session.namespace,
        shadow: false,
        adapter: Some(AdapterConfig {
            adapter_listener_name: ADAPTER_LISTENER_NAME,
            breakpoints: &breakpoints,
        }),
    };
    let _ = remove_dir_all(&minecraft_session.output_path).await;
    generate_debug_datapack(
        &minecraft_session.datapack,
        &minecraft_session.output_path,
        &config,
    )
    .await
    .map_err(|e| PartialErrorResponse::new(format!("Failed to generate debug datapack: {}", e)))?;
    Ok(())
}

pub fn contains_breakpoint(
    breakpoints: &MultiMap<ResourceLocation, LocalBreakpoint>,
    breakpoint: &McfunctionBreakpoint<String>,
) -> bool {
    let breakpoints = breakpoints.get_vec(&breakpoint.function);
    if let Some(breakpoints) = breakpoints {
        breakpoints
            .iter()
            .find(|it| it.line_number == breakpoint.line_number)
            .is_some()
    } else {
        false
    }
}

pub fn events_between_tags(
    stream: UnboundedReceiverStream<LogEvent>,
    start_tag: &str,
    stop_tag: &str,
) -> impl Stream<Item = LogEvent> {
    let added_start_tag = format!("Added tag '{1}' to {0}", ADAPTER_LISTENER_NAME, start_tag);
    let added_end_tag = format!("Added tag '{1}' to {0}", ADAPTER_LISTENER_NAME, stop_tag);
    stream
        .skip_while(move |event| {
            ready(event.executor != ADAPTER_LISTENER_NAME && event.message != added_start_tag)
        })
        .skip(1) // Skip start tag
        .take_while(move |event| {
            ready(event.executor != ADAPTER_LISTENER_NAME && event.message != added_end_tag)
        })
}

pub struct McfunctionBreakpoint<S: AsRef<str>> {
    pub function: ResourceLocationRef<S>,
    pub line_number: usize,
}

pub struct McfunctionBreakpointTag<S: AsRef<str>>(pub McfunctionBreakpoint<S>);

impl FromStr for McfunctionBreakpointTag<String> {
    type Err = ();

    fn from_str(breakpoint_tag: &str) -> Result<Self, Self::Err> {
        // -ns-+-orig_ns-+-orig_fn-+-line_number-
        if let [orig_ns, orig_fn @ .., line_number] =
            breakpoint_tag.split('+').collect::<Vec<_>>().as_slice()
        {
            let function = ResourceLocation::new(orig_ns, &orig_fn.join("/"));
            let line_number = line_number.parse::<usize>().map_err(|_| ())?;
            Ok(McfunctionBreakpointTag(McfunctionBreakpoint {
                function,
                line_number,
            }))
        } else {
            Err(())
        }
    }
}

impl<S: AsRef<str>> Display for McfunctionBreakpointTag<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}+{}+{}",
            self.0.function.namespace(),
            self.0.function.path().replace("/", "+"),
            self.0.line_number
        )
    }
}
