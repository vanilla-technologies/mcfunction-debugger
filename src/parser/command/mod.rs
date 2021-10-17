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

pub mod argument;
pub mod resource_location;

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{Display, Write},
    u32, usize,
};

use self::argument::{Argument, ArgumentParser};

pub struct CommandParser {
    commands: HashMap<String, Command>,
}

impl CommandParser {
    pub fn default() -> Result<CommandParser, serde_json::Error> {
        let json = include_str!("commands.json");
        CommandParser::from_str(json)
    }

    pub fn from_str(json: &str) -> serde_json::Result<CommandParser> {
        let root_node: RootNode = serde_json::from_str(json)?;
        Ok(CommandParser {
            commands: root_node.children,
        })
    }

    pub fn parse<'l>(&'l self, command: &'l str) -> CommandParserResult<'l> {
        let mut parsed_nodes = Vec::new();
        let mut commands = &self.commands;

        let mut index = 0;
        loop {
            let (command_spec, parsed_node, parsed_len) =
                match CommandParser::parse_from(command, index, commands) {
                    Ok(ok) => ok,
                    Err(message) => {
                        return CommandParserResult {
                            parsed_nodes,
                            error: Some(CommandParserError {
                                message,
                                command,
                                index,
                            }),
                        }
                    }
                };
            parsed_nodes.push(parsed_node);
            index += parsed_len;

            if index >= command.len() {
                if command_spec.executable() {
                    return CommandParserResult {
                        parsed_nodes,
                        error: None,
                    };
                } else {
                    return CommandParserResult {
                        parsed_nodes,
                        error: Some(CommandParserError {
                            message: "Incomplete command".to_string(),
                            command,
                            index,
                        }),
                    };
                }
            } else {
                const SPACE: char = ' ';
                if !command[index..].starts_with(SPACE) {
                    return CommandParserResult {
                        parsed_nodes,
                        error: Some(CommandParserError {
                            message:
                                "Expected whitespace to end one argument, but found trailing data"
                                    .to_string(),
                            command,
                            index,
                        }),
                    };
                }
                index += SPACE.len_utf8();

                commands = command_spec.children();
                let redirect = match command_spec.redirect() {
                    Ok(ok) => ok,
                    Err(message) => {
                        return CommandParserResult {
                            parsed_nodes,
                            error: Some(CommandParserError {
                                message,
                                command,
                                index,
                            }),
                        }
                    }
                };
                if let Some(redirect) = redirect {
                    if let Some(command) = self.commands.get(redirect) {
                        parsed_nodes.push(ParsedNode::Redirect(redirect));
                        commands = command.children();
                    } else {
                        return CommandParserResult {
                            parsed_nodes,
                            error: Some(CommandParserError {
                                message: format!("Failed to resolve redirect {}", redirect),
                                command,
                                index,
                            }),
                        };
                    }
                } else if commands.is_empty() {
                    if !command_spec.executable() {
                        // Special case for execute run which has no redirect to root for some reason
                        commands = &self.commands;
                    } else {
                        return CommandParserResult {
                            parsed_nodes,
                            error: Some(CommandParserError {
                                message: "Incorrect argument for command".to_string(),
                                command,
                                index,
                            }),
                        };
                    }
                }
            }
        }
    }

    fn parse_from<'l>(
        command: &'l str,
        index: usize,
        commands: &'l HashMap<String, Command>,
    ) -> Result<(&'l Command, ParsedNode<'l>, usize), String> {
        // Try to parse as literal
        let string = &command[index..];
        let len = string.find(' ').unwrap_or(string.len());
        let literal = &string[..len];
        let command_spec = commands
            .iter()
            .find(|(name, command)| {
                name.as_str() == literal && matches!(command, Command::Literal { .. })
            })
            .map(|(_name, command)| command);
        if let Some(command) = command_spec {
            Ok((command, ParsedNode::Literal { literal, index }, len))
        } else {
            // try to parse as argument
            let mut parsed_arguments = commands
                .iter()
                .filter_map(|(name, command)| match command {
                    Command::Literal { .. } => None,
                    Command::Argument { parser, .. } => Some((name, command, parser)),
                })
                .map(|(name, command, parser)| (name, command, parser.parse(string)))
                .collect::<Vec<_>>();
            // Prefer longest successful parsed
            parsed_arguments.sort_by_key(|(_name, _command, r)| match r {
                Ok((_argument, len)) => -(*len as isize),
                Err(_) => 1,
            });
            let (name, command_spec, result) = parsed_arguments
                .into_iter()
                .next()
                .ok_or("Unknown command".to_string())?;
            let (argument, len) = result?;
            let parsed = ParsedNode::Argument {
                name,
                argument,
                index,
                len,
            };
            Ok((command_spec, parsed, len))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommandParserResult<'l> {
    pub parsed_nodes: Vec<ParsedNode<'l>>,
    pub error: Option<CommandParserError<'l>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommandParserError<'l> {
    pub message: String,
    pub command: &'l str,
    pub index: usize,
}

impl Display for CommandParserError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:\n{}\n", self.message, self.command)?;
        for _ in 0..self.index {
            f.write_char(' ')?;
        }
        f.write_char('^')
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParsedNode<'l> {
    Redirect(&'l str),
    Literal {
        literal: &'l str,
        index: usize,
    },
    Argument {
        name: &'l str,
        argument: Argument<'l>,
        index: usize,
        len: usize,
    },
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type", rename = "root")]
struct RootNode {
    children: HashMap<String, Command>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Literal {
        #[serde(flatten)]
        node: Node,
    },
    Argument {
        #[serde(flatten)]
        node: Node,
        #[serde(flatten)]
        parser: ArgumentParser,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Node {
    #[serde(default)]
    pub children: HashMap<String, Command>,
    #[serde(default)]
    pub executable: bool,
    #[serde(default)]
    pub redirect: Vec<String>,
}

impl Command {
    pub fn children(&self) -> &HashMap<String, Command> {
        match self {
            Command::Literal { node, .. } => &node.children,
            Command::Argument { node, .. } => &node.children,
        }
    }

    pub fn executable(&self) -> bool {
        match self {
            Command::Literal { node, .. } => node.executable,
            Command::Argument { node, .. } => node.executable,
        }
    }

    pub fn redirect(&self) -> Result<Option<&String>, String> {
        let redirect = match self {
            Command::Literal { node, .. } => &node.redirect,
            Command::Argument { node, .. } => &node.redirect,
        };
        if redirect.len() > 1 {
            Err(format!("Multi redirect is not supported: {:?}", redirect))
        } else {
            Ok(redirect.first())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json() {
        // when:
        let actual = &CommandParser::default().unwrap().commands;

        // then:
        assert!(
            actual.contains_key("execute"),
            "Expected actual to contain key 'execute': {:#?}",
            actual
        );
    }

    #[test]
    fn test_serialize() {
        // when:
        let root = RootNode {
            children: HashMap::new(),
        };

        let actual = serde_json::to_string(&root).unwrap();

        // then:
        assert_eq!(actual, r#"{"type":"root","children":{}}"#);
    }
}
