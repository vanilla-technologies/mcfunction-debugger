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
use std::{convert::TryFrom, usize};

#[derive(Debug, PartialEq)]
pub enum Line {
    Breakpoint,
    FunctionCall {
        name: ResourceLocation,
        anchor: Option<MinecraftEntityAnchor>,
        execute_as: bool,
        selectors: Vec<usize>,
    },
    Schedule {
        schedule_start: usize,
        function: ResourceLocation,
        operation: ScheduleOperation,
        selectors: Vec<usize>,
    },
    OtherCommand {
        selectors: Vec<usize>,
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
                selectors: Vec::new(),
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
    let mut selectors = Vec::new();
    let mut maybe_anchor: Option<MinecraftEntityAnchor> = None;
    let mut execute_as = false;

    while let Some((head, tail)) = nodes.split_first() {
        nodes = tail;
        match head {
            ParsedNode::Argument {
                argument:
                    Argument::MinecraftEntity(..)
                    | Argument::MinecraftScoreHolder(MinecraftScoreHolder::Selector(..)),
                index,
            } => {
                selectors.push(*index);
            }
            ParsedNode::Argument {
                argument:
                    Argument::MinecraftMessage(MinecraftMessage {
                        selectors: message_selectors,
                        ..
                    }),
                index,
            } => {
                selectors.extend(
                    message_selectors
                        .iter()
                        .map(|(_selector, start, _end)| index + start),
                );
            }
            ParsedNode::Literal {
                literal: "execute", ..
            }
            | ParsedNode::Redirect("execute") => {
                if let Some((
                    ParsedNode::Literal {
                        literal: "anchored",
                        ..
                    },
                    tail,
                )) = tail.split_first()
                {
                    if let Some(ParsedNode::Argument {
                        argument: Argument::MinecraftEntityAnchor(anchor),
                        ..
                    }) = tail.first()
                    {
                        maybe_anchor = Some(*anchor);
                    }
                }
                if let Some((ParsedNode::Literal { literal: "as", .. }, _tail)) = tail.split_first()
                {
                    execute_as = true;
                }
            }
            ParsedNode::Literal {
                literal: "function",
                ..
            } => {
                if let Some(ParsedNode::Argument {
                    argument: Argument::MinecraftFunction(function),
                    ..
                }) = tail.first()
                {
                    return (
                        Line::FunctionCall {
                            name: function.to_owned(),
                            anchor: maybe_anchor,
                            execute_as,
                            selectors,
                        },
                        error,
                    );
                }
            }
            ParsedNode::Literal {
                literal: "schedule",
                index,
            } => {
                if let Some((
                    ParsedNode::Literal {
                        literal: "function",
                        ..
                    },
                    tail,
                )) = tail.split_first()
                {
                    if let Some((
                        ParsedNode::Argument {
                            argument: Argument::MinecraftFunction(function),
                            ..
                        },
                        tail,
                    )) = tail.split_first()
                    {
                        if let Some((
                            ParsedNode::Argument {
                                argument: Argument::MinecraftTime(time),
                                ..
                            },
                            tail,
                        )) = tail.split_first()
                        {
                            let operation = match tail.first() {
                                Some(ParsedNode::Literal {
                                    literal: "append", ..
                                }) => ScheduleOperation::APPEND { time: time.clone() },
                                None
                                | Some(ParsedNode::Literal {
                                    literal: "replace", ..
                                }) => ScheduleOperation::REPLACE { time: time.clone() },
                                _ => return (Line::OtherCommand { selectors }, None),
                            };

                            return (
                                Line::Schedule {
                                    schedule_start: *index,
                                    function: function.to_owned(),
                                    operation,
                                    selectors,
                                },
                                error,
                            );
                        }
                    }
                }
                if let Some((
                    ParsedNode::Literal {
                        literal: "clear", ..
                    },
                    tail,
                )) = tail.split_first()
                {
                    if let Some(ParsedNode::Argument {
                        argument: Argument::BrigadierString(string),
                        ..
                    }) = tail.first()
                    {
                        // TODO Handle invalid characters in NamespacedName
                        if let Ok(function) = ResourceLocationRef::try_from(*string) {
                            return (
                                Line::Schedule {
                                    schedule_start: *index,
                                    function: function.to_owned(),
                                    operation: ScheduleOperation::CLEAR,
                                    selectors,
                                },
                                error,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }
    (Line::OtherCommand { selectors }, error)
}

#[cfg(test)]
mod tests;
