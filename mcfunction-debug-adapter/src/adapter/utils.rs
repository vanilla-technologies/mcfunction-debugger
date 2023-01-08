// mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any
// Minecraft mods.
//
// © Copyright (C) 2021-2023 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
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
    adapter::{MinecraftSession, LISTENER_NAME},
    error::PartialErrorResponse,
};
use debug_adapter_protocol::events::StoppedEventReason;
use futures::Stream;
use mcfunction_debugger::{
    generate_debug_datapack,
    parser::command::resource_location::{ResourceLocation, ResourceLocationRef},
    AdapterConfig, BreakpointKind, Config, LocalBreakpoint, StoppedReason,
};
use minect::log::{LogEvent, SummonNamedEntityOutput};
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
            if !contains_breakpoint(&breakpoints, &Position::from_breakpoint(key.clone(), value)) {
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
    position: &Position,
) -> bool {
    get_breakpoint_kind(breakpoints, position)
        .map(|it| it.can_resume())
        .unwrap_or(false)
}

pub(crate) fn contains_breakpoint(
    breakpoints: &MultiMap<ResourceLocation, LocalBreakpoint>,
    position: &Position,
) -> bool {
    get_breakpoint_kind(breakpoints, position).is_some()
}

pub(crate) fn get_breakpoint_kind<'l>(
    breakpoints: &'l MultiMap<ResourceLocation, LocalBreakpoint>,
    position: &Position,
) -> Option<&'l BreakpointKind> {
    if let Some(breakpoints) = breakpoints.get_vec(&position.function) {
        breakpoints
            .iter()
            .filter(|it| it.line_number == position.line_number)
            .filter(|it| SuspensionPositionInLine::from(&it.kind) == position.position_in_line)
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SuspensionPositionInLine {
    Breakpoint,
    AfterFunction,
}
impl From<&BreakpointKind> for SuspensionPositionInLine {
    fn from(value: &BreakpointKind) -> Self {
        match value {
            BreakpointKind::Normal => SuspensionPositionInLine::Breakpoint,
            BreakpointKind::Invalid => SuspensionPositionInLine::Breakpoint,
            BreakpointKind::Continue { after_function }
            | BreakpointKind::Step { after_function, .. } => {
                if *after_function {
                    SuspensionPositionInLine::AfterFunction
                } else {
                    SuspensionPositionInLine::Breakpoint
                }
            }
        }
    }
}
impl FromStr for SuspensionPositionInLine {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "breakpoint" => Ok(SuspensionPositionInLine::Breakpoint),
            "after_function" => Ok(SuspensionPositionInLine::AfterFunction),
            _ => Err(()),
        }
    }
}
impl Display for SuspensionPositionInLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SuspensionPositionInLine::Breakpoint => write!(f, "breakpoint"),
            SuspensionPositionInLine::AfterFunction => write!(f, "after_function"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Position {
    pub(crate) function: ResourceLocation,
    pub(crate) line_number: usize,
    pub(crate) position_in_line: SuspensionPositionInLine,
}
impl Position {
    fn from_breakpoint(function: ResourceLocation, breakpoint: &LocalBreakpoint) -> Position {
        Position {
            function,
            line_number: breakpoint.line_number,
            position_in_line: (&breakpoint.kind).into(),
        }
    }
}
impl FromStr for Position {
    type Err = ();

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        fn from_str_inner(string: &str) -> Option<Position> {
            let last_delimiter = string.rfind('+')?;
            let function = &string[..last_delimiter];
            let position = &string[last_delimiter + 1..];

            let (line_number, position_in_line) = position.split_once('_')?;

            let line_number = line_number.parse().ok()?;
            let position_in_line = position_in_line.parse().ok()?;

            if let [orig_ns, orig_fn @ ..] = function.split('+').collect::<Vec<_>>().as_slice() {
                let function = ResourceLocation::new(orig_ns, &orig_fn.join("/"));
                Some(Position {
                    function,
                    line_number,
                    position_in_line,
                })
            } else {
                None
            }
        }
        from_str_inner(string).ok_or(())
    }
}
impl Display for Position {
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

pub(crate) struct StoppedEvent {
    pub(crate) reason: StoppedReason,
    pub(crate) position: Position,
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

#[derive(Clone, Debug)]
pub(crate) struct StackFrameLocation {
    pub(crate) function: ResourceLocation,
    pub(crate) line_number: usize,
    pub(crate) column_number: usize,
}
impl StackFrameLocation {
    pub(crate) fn parse(executor: &str) -> Option<StackFrameLocation> {
        let has_column = 3 == executor.bytes().filter(|b| *b == b':').count();
        let (function_line, column_number) = if has_column {
            let last_delimiter = executor.rfind(':')?;
            (
                &executor[..last_delimiter],
                executor[last_delimiter + 1..].parse().ok()?,
            )
        } else {
            (executor, 1)
        };
        let function_line = McfunctionLineNumber::parse(function_line, ":")?;
        Some(StackFrameLocation {
            function: function_line.function,
            line_number: function_line.line_number,
            column_number,
        })
    }

    pub(crate) fn get_name(&self) -> String {
        format!("{}:{}", self.function, self.line_number)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct McfunctionLineNumber<S: AsRef<str>> {
    pub function: ResourceLocationRef<S>,
    pub line_number: usize,
}

impl McfunctionLineNumber<String> {
    pub(crate) fn parse(string: &str, seperator: &str) -> Option<Self> {
        if let [orig_ns, orig_fn @ .., line_number] =
            string.split(seperator).collect::<Vec<_>>().as_slice()
        {
            let function = ResourceLocation::new(orig_ns, &orig_fn.join("/"));
            let line_number = line_number.parse::<usize>().ok()?;
            Some(McfunctionLineNumber {
                function,
                line_number,
            })
        } else {
            None
        }
    }
}
