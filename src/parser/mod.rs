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

pub mod command;

use self::command::{
    argument::{
        Argument, MinecraftEntityAnchor, MinecraftMessage, MinecraftScoreHolder, MinecraftTime,
    },
    resource_location::{ResourceLocation, ResourceLocationRef},
    CommandParser, CommandParserError, CommandParserResult, ParsedNode,
};
use log::debug;
use std::{collections::BTreeSet, convert::TryFrom, usize};

#[derive(Debug, PartialEq)]
pub enum Line {
    Breakpoint,
    FunctionCall {
        name: ResourceLocation,
        anchor: Option<MinecraftEntityAnchor>,
        selectors: BTreeSet<usize>,
    },
    OptionalSelectorCommand {
        missing_selector: usize,
        selectors: BTreeSet<usize>,
    },
    Schedule {
        schedule_start: usize,
        function: ResourceLocation,
        operation: ScheduleOperation,
        selectors: BTreeSet<usize>,
    },
    OtherCommand {
        selectors: BTreeSet<usize>,
    },
}

#[derive(Debug, PartialEq)]
pub enum ScheduleOperation {
    APPEND { time: MinecraftTime },
    CLEAR,
    REPLACE { time: MinecraftTime },
}

pub fn parse_line(parser: &CommandParser, line: &str) -> Line {
    let (line, error) = parse_line_internal(parser, line);
    if let Some(error) = error {
        debug!("Failed to parse command. {}", error);
    }
    line
}

fn parse_line_internal<'l>(
    parser: &'l CommandParser,
    line: &'l str,
) -> (Line, Option<CommandParserError<'l>>) {
    let line = line.trim();
    if line == "# breakpoint" {
        (Line::Breakpoint, None)
    } else if line.is_empty() || line.starts_with('#') {
        (
            Line::OtherCommand {
                selectors: BTreeSet::new(),
            },
            None,
        )
    } else {
        parse_command(parser, line)
    }
}

fn parse_command<'l>(
    parser: &'l CommandParser,
    command: &'l str,
) -> (Line, Option<CommandParserError<'l>>) {
    let CommandParserResult {
        parsed_nodes,
        error,
    } = parser.parse(command);
    let mut nodes = parsed_nodes.as_slice();
    let mut selectors = BTreeSet::new();
    let mut maybe_anchor: Option<MinecraftEntityAnchor> = None;

    while let [_, tail @ ..] = nodes {
        match nodes {
            [ParsedNode::Argument {
                argument:
                    Argument::MinecraftEntity(..)
                    | Argument::MinecraftScoreHolder(MinecraftScoreHolder::Selector(..)),
                index,
                ..
            }, ..] => {
                selectors.insert(*index);
            }

            [ParsedNode::Argument {
                argument:
                    Argument::MinecraftMessage(MinecraftMessage {
                        selectors: message_selectors,
                        ..
                    }),
                index,
                ..
            }, ..] => {
                selectors.extend(
                    message_selectors
                        .iter()
                        .map(|(_selector, start, _end)| index + start),
                );
            }

            [ParsedNode::Literal {
                literal: "execute", ..
            }
            | ParsedNode::Redirect("execute"), ParsedNode::Literal {
                literal: "anchored",
                ..
            }, ParsedNode::Argument {
                argument: Argument::MinecraftEntityAnchor(anchor),
                ..
            }, ..] => {
                maybe_anchor = Some(*anchor);
            }

            _ => {}
        }

        nodes = tail;
    }

    if error.is_none() {
        if let Some(name) = as_function_call(&parsed_nodes) {
            return (
                Line::FunctionCall {
                    name,
                    anchor: maybe_anchor,
                    selectors,
                },
                None,
            );
        }

        if let Some((schedule_start, function, operation)) = as_schedule(&parsed_nodes) {
            return (
                Line::Schedule {
                    schedule_start,
                    function: function.to_owned(),
                    operation,
                    selectors,
                },
                None,
            );
        }

        if let Some(missing_selector) = find_missing_selector(&parsed_nodes) {
            return (
                Line::OptionalSelectorCommand {
                    missing_selector,
                    selectors,
                },
                None,
            );
        }
    }

    (Line::OtherCommand { selectors }, error)
}

fn as_function_call(nodes: &[ParsedNode]) -> Option<ResourceLocation> {
    if let [.., ParsedNode::Literal {
        literal: "function",
        ..
    }, ParsedNode::Argument {
        argument: Argument::MinecraftFunction(function),
        ..
    }] = nodes
    {
        Some(function.to_owned())
    } else {
        None
    }
}

fn as_schedule(mut nodes: &[ParsedNode]) -> Option<(usize, ResourceLocation, ScheduleOperation)> {
    while let [_, tail @ ..] = nodes {
        match nodes {
            [ParsedNode::Literal {
                literal: "schedule",
                index,
                ..
            }, ParsedNode::Literal {
                literal: "function",
                ..
            }, ParsedNode::Argument {
                argument: Argument::MinecraftFunction(function),
                ..
            }, ParsedNode::Argument {
                argument: Argument::MinecraftTime(time),
                ..
            }, tail @ ..] => {
                let op = match tail {
                    [ParsedNode::Literal {
                        literal: "append", ..
                    }] => Some(ScheduleOperation::APPEND { time: time.clone() }),
                    []
                    | [ParsedNode::Literal {
                        literal: "replace", ..
                    }] => Some(ScheduleOperation::REPLACE { time: time.clone() }),
                    _ => None,
                };
                if let Some(op) = op {
                    return Some((*index, function.to_owned(), op));
                }
            }

            [ParsedNode::Literal {
                literal: "schedule",
                index,
                ..
            }, ParsedNode::Literal {
                literal: "clear", ..
            }, ParsedNode::Argument {
                argument: Argument::BrigadierString(string),
                ..
            }] => {
                // TODO Handle invalid characters in NamespacedName
                if let Ok(function) = ResourceLocationRef::try_from(*string) {
                    return Some((*index, function.to_owned(), ScheduleOperation::CLEAR));
                }
            }
            _ => {}
        }

        nodes = tail;
    }

    None
}

fn find_missing_selector(tail: &[ParsedNode]) -> Option<usize> {
    match tail {
        [.., ParsedNode::Literal {
            literal: kill @ "kill",
            index,
        }] => Some(index + kill.len()),

        [.., ParsedNode::Literal {
            literal: "team", ..
        }, ParsedNode::Literal {
            literal: "join", ..
        }, ParsedNode::Argument {
            argument: Argument::MinecraftTeam(..),
            index,
            len,
            ..
        }] => Some(index + len),

        [.., ParsedNode::Redirect("teleport")
        | ParsedNode::Literal {
            literal: "teleport",
            ..
        }, ParsedNode::Argument {
            name: "destination",
            argument: Argument::MinecraftEntity(..),
            index,
            ..
        }
        | ParsedNode::Argument {
            name: "location",
            argument: Argument::MinecraftVec3(..),
            index,
            ..
        }] => Some(index - 1),

        _ => None,
    }
}

#[cfg(test)]
mod tests;
