// McFunction-Debugger is a debugger for Minecraft's *.mcfunction files that does not require any
// Minecraft mods.
//
// © Copyright (C) 2021-2023 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
//
// This file is part of McFunction-Debugger.
//
// McFunction-Debugger is free software: you can redistribute it and/or modify it under the terms of
// the GNU General Public License as published by the Free Software Foundation, either version 3 of
// the License, or (at your option) any later version.
//
// McFunction-Debugger is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with McFunction-Debugger.
// If not, see <http://www.gnu.org/licenses/>.

use crate::{
    adapter::{MinecraftSession, LISTENER_NAME},
    error::PartialErrorResponse,
};
use debug_adapter_protocol::{
    events::StoppedEventReason,
    types::{Source, StackFrame},
};
use futures::Stream;
use mcfunction_debugger::{
    config::{
        adapter::{
            AdapterConfig, BreakpointKind, BreakpointPositionInLine, LocalBreakpoint,
            LocalBreakpointPosition,
        },
        Config,
    },
    generate_debug_datapack,
    parser::command::resource_location::ResourceLocation,
    StoppedReason,
};
use minect::{command::SummonNamedEntityOutput, log::LogEvent};
use multimap::MultiMap;
use std::{fmt::Display, path::Path, str::FromStr};
use tokio::fs::remove_dir_all;
use tokio_stream::StreamExt;

pub fn parse_function_path(path: &Path) -> Result<(&Path, ResourceLocation), String> {
    let datapack = find_parent_datapack(path).ok_or_else(|| {
        format!(
            "does not denote a path in a datapack directory with a pack.mcmeta file: {}",
            &path.display()
        )
    })?;
    let data_path = path.strip_prefix(datapack.join("data")).map_err(|_| {
        format!(
            "does not denote a path in the data directory of datapack {}: {}",
            &datapack.display(),
            &path.display()
        )
    })?;
    let function = get_function_name(data_path, &path)?;
    Ok((datapack, function))
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
    temporary_breakpoints: &MultiMap<ResourceLocation, LocalBreakpoint>,
) -> Result<(), PartialErrorResponse> {
    let mut breakpoints = breakpoints.clone();

    // Add all generated breakpoints that are not at the same position as user breakpoints
    for (key, values) in temporary_breakpoints.iter_all() {
        for value in values {
            if !contains_breakpoint(
                &breakpoints,
                &BreakpointPosition::from_breakpoint(key.clone(), &value.position),
            ) {
                breakpoints.insert(key.clone(), value.clone());
            }
        }
    }

    let config = Config {
        namespace: &minecraft_session.namespace,
        shadow: false,
        adapter: Some(AdapterConfig {
            adapter_listener_name: LISTENER_NAME,
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

pub(crate) fn can_resume_from(
    breakpoints: &MultiMap<ResourceLocation, LocalBreakpoint>,
    position: &BreakpointPosition,
) -> bool {
    get_breakpoint_kind(breakpoints, position)
        .map(|it| it.can_resume())
        .unwrap_or(false)
}

pub(crate) fn contains_breakpoint(
    breakpoints: &MultiMap<ResourceLocation, LocalBreakpoint>,
    position: &BreakpointPosition,
) -> bool {
    get_breakpoint_kind(breakpoints, position).is_some()
}

pub(crate) fn get_breakpoint_kind<'l>(
    breakpoints: &'l MultiMap<ResourceLocation, LocalBreakpoint>,
    position: &BreakpointPosition,
) -> Option<&'l BreakpointKind> {
    if let Some(breakpoints) = breakpoints.get_vec(&position.function) {
        breakpoints
            .iter()
            .filter(|it| it.position.line_number == position.line_number)
            .filter(|it| it.position.position_in_line == position.position_in_line)
            .map(|it| &it.kind)
            .next()
    } else {
        None
    }
}

pub(crate) fn events_between<'l>(
    events: impl Stream<Item = LogEvent> + 'l,
    start: &'l str,
    stop: &'l str,
) -> impl Stream<Item = LogEvent> + 'l {
    events
        .skip_while(move |event| !is_summon_output(event, start))
        .skip(1) // Skip start tag
        .take_while(move |event| !is_summon_output(event, stop))
}
fn is_summon_output(event: &LogEvent, name: &str) -> bool {
    event.executor == LISTENER_NAME
        && event
            .output
            .parse::<SummonNamedEntityOutput>()
            .ok()
            .filter(|output| output.name == name)
            .is_some()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BreakpointPosition {
    pub(crate) function: ResourceLocation,
    pub(crate) line_number: usize,
    pub(crate) position_in_line: BreakpointPositionInLine,
}
impl BreakpointPosition {
    pub(crate) fn from_breakpoint(
        function: ResourceLocation,
        position: &LocalBreakpointPosition,
    ) -> BreakpointPosition {
        BreakpointPosition {
            function,
            line_number: position.line_number,
            position_in_line: position.position_in_line,
        }
    }
}
impl FromStr for BreakpointPosition {
    type Err = ();

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        fn from_str_inner(string: &str) -> Option<BreakpointPosition> {
            let (function, position) = string.rsplit_once('+')?;
            let (line_number, position_in_line) = position.split_once('_')?;

            let function = parse_resource_location(function, '+')?;
            let line_number = line_number.parse().ok()?;
            let position_in_line = position_in_line.parse().ok()?;

            Some(BreakpointPosition {
                function,
                line_number,
                position_in_line,
            })
        }
        from_str_inner(string).ok_or(())
    }
}
impl Display for BreakpointPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}+{}+{}_{}",
            self.function.namespace(),
            self.function.path().replace("/", "+"),
            self.line_number,
            self.position_in_line,
        )
    }
}

pub(crate) struct StoppedData {
    pub(crate) position: BreakpointPosition,
    pub(crate) stack_trace: Vec<McfunctionStackFrame>,
}

pub(crate) struct StoppedEvent {
    pub(crate) reason: StoppedReason,
    pub(crate) position: BreakpointPosition,
}
impl FromStr for StoppedEvent {
    type Err = ();

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        fn from_str_inner(string: &str) -> Option<StoppedEvent> {
            let string = string.strip_prefix("stopped+")?;
            let (reason, position) = string.split_once('+')?;
            let reason = reason.parse().ok()?;
            let position = position.parse().ok()?;
            Some(StoppedEvent { reason, position })
        }
        from_str_inner(string).ok_or(())
    }
}
pub(crate) fn to_stopped_event_reason(reason: StoppedReason) -> StoppedEventReason {
    match reason {
        StoppedReason::Breakpoint => StoppedEventReason::Breakpoint,
        StoppedReason::Step => StoppedEventReason::Step,
    }
}

pub(crate) struct McfunctionStackFrame {
    pub(crate) id: i32,
    pub(crate) location: SourceLocation,
}
impl McfunctionStackFrame {
    pub(crate) fn to_stack_frame(
        &self,
        datapack: impl AsRef<Path>,
        line_offset: usize,
        column_offset: usize,
    ) -> StackFrame {
        let path = datapack
            .as_ref()
            .join("data")
            .join(self.location.function.mcfunction_path())
            .display()
            .to_string();
        StackFrame::builder()
            .id(self.id)
            .name(self.location.get_name())
            .source(Some(Source::builder().path(Some(path)).build()))
            .line((self.location.line_number - line_offset) as i32)
            .column((self.location.column_number - column_offset) as i32)
            .build()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SourceLocation {
    pub(crate) function: ResourceLocation,
    pub(crate) line_number: usize,
    pub(crate) column_number: usize,
}
impl FromStr for SourceLocation {
    type Err = ();

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        fn from_str_inner(string: &str) -> Option<SourceLocation> {
            let has_column = 3 == string.bytes().filter(|b| *b == b':').count();
            let (function_line_number, column_number) = if has_column {
                let (function_line_number, column_number) = string.rsplit_once(':')?;
                let column_number = column_number.parse().ok()?;
                (function_line_number, column_number)
            } else {
                (string, 1)
            };

            let (function, line_number) = function_line_number.rsplit_once(':')?;
            let function = parse_resource_location(function, ':')?;
            let line_number = line_number.parse().ok()?;

            Some(SourceLocation {
                function,
                line_number,
                column_number,
            })
        }
        from_str_inner(string).ok_or(())
    }
}
impl SourceLocation {
    pub(crate) fn get_name(&self) -> String {
        format!("{}:{}", self.function, self.line_number)
    }
}

fn parse_resource_location(function: &str, seperator: char) -> Option<ResourceLocation> {
    if let [orig_ns, orig_fn @ ..] = function.split(seperator).collect::<Vec<_>>().as_slice() {
        Some(ResourceLocation::new(orig_ns, &orig_fn.join("/")))
    } else {
        None
    }
}
