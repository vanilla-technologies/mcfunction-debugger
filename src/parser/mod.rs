pub mod commands;

use self::commands::{
    Argument, CommandParser, CommandParserError, CommandParserResult, MinecraftEntityAnchor,
    MinecraftMessage, MinecraftScoreHolder, MinecraftTime, NamespacedName, NamespacedNameRef,
    ParsedNode,
};
use log::warn;
use std::usize;

#[derive(Debug, PartialEq)]
pub enum Line {
    Breakpoint,
    FunctionCall {
        name: NamespacedName,
        anchor: Option<MinecraftEntityAnchor>,
        execute_as: bool,
        selectors: Vec<usize>,
    },
    Schedule {
        schedule_start: usize,
        function: NamespacedName,
        time: Option<MinecraftTime>,
        category: Option<String>,
        selectors: Vec<usize>,
    },
    OtherCommand {
        selectors: Vec<usize>,
    },
}

pub fn parse_line(parser: &CommandParser, line: &str) -> Line {
    let (line, error) = parse_line_internal(parser, line);
    if let Some(error) = error {
        warn!("Failed to parse command. {}", error);
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
                            return (
                                Line::Schedule {
                                    schedule_start: *index,
                                    function: function.to_owned(),
                                    time: Some(time.clone()),
                                    category: if let Some(ParsedNode::Literal {
                                        literal: category,
                                        ..
                                    }) = tail.first()
                                    {
                                        Some(category.to_string())
                                    } else {
                                        None
                                    },
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
                        if let Some(function) = NamespacedNameRef::from(string) {
                            return (
                                Line::Schedule {
                                    schedule_start: *index,
                                    function: function.to_owned(),
                                    time: None,
                                    category: Some("clear".to_string()),
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
