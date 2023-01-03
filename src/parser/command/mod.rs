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

pub mod argument;
pub mod resource_location;

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt::{Display, Write},
    u32, usize,
};

use self::argument::{Argument, ArgumentParser};

pub struct CommandParser {
    specs: BTreeMap<String, CommandSpec>,
}

impl CommandParser {
    pub fn default() -> Result<CommandParser, serde_json::Error> {
        let json = include_str!("commands.json");
        CommandParser::from_str(json)
    }

    pub fn from_str(json: &str) -> serde_json::Result<CommandParser> {
        let root_node: RootNode = serde_json::from_str(json)?;
        Ok(CommandParser {
            specs: root_node.children,
        })
    }

    pub fn parse<'l>(&'l self, command: &'l str) -> CommandParserResult<'l> {
        self.parse_from_specs(command, 0, &self.specs)
    }

    fn parse_from_specs<'l>(
        &'l self,
        command: &'l str,
        index: usize,
        specs: &'l BTreeMap<String, CommandSpec>,
    ) -> CommandParserResult<'l> {
        let parsed = Self::find_relevant_commands(command, index, specs)
            .into_iter()
            .map(|(name, spec)| (self.parse_from_single_spec(name, spec, command, index)))
            .collect::<Vec<_>>();

        let only_errors = parsed.iter().all(|parsed| parsed.error.is_some());
        if only_errors {
            // Return deepest error
            parsed
                .into_iter()
                .max_by_key(|result| result.parsed_nodes.len())
                .unwrap_or(CommandParserResult {
                    parsed_nodes: Vec::new(),
                    error: Some(CommandParserError {
                        message: "Incorrect argument for command".to_string(),
                        command,
                        index,
                    }),
                })
        } else {
            // Return first non error
            parsed
                .into_iter()
                .filter(|parsed| parsed.error.is_none())
                .next()
                .unwrap()
        }
    }

    /// If the next part can be parsed as a literal, arguments should be ignored.
    fn find_relevant_commands<'l>(
        command: &'l str,
        index: usize,
        specs: &'l BTreeMap<String, CommandSpec>,
    ) -> Vec<(&'l String, &'l CommandSpec)> {
        let string = &command[index..];
        let literal_len = string.find(' ').unwrap_or(string.len());
        let literal = &string[..literal_len];
        if let Some((name, command)) = Self::find_literal_command(literal, specs) {
            vec![(name, command)]
        } else {
            specs
                .iter()
                .filter(|(_name, spec)| matches!(spec, CommandSpec::Argument { .. }))
                .collect::<Vec<_>>()
        }
    }

    fn find_literal_command<'l>(
        literal: &str,
        specs: &'l BTreeMap<String, CommandSpec>,
    ) -> Option<(&'l String, &'l CommandSpec)> {
        specs
            .iter()
            .find(|(name, spec)| *name == literal && matches!(spec, CommandSpec::Literal { .. }))
    }

    fn parse_from_single_spec<'l>(
        &'l self,
        name: &'l str,
        spec: &'l CommandSpec,
        command: &'l str,
        mut index: usize,
    ) -> CommandParserResult<'l> {
        let mut parsed_nodes = Vec::new();

        macro_rules! Ok {
            () => {
                CommandParserResult {
                    parsed_nodes,
                    error: None,
                }
            };
        }
        macro_rules! Err {
            ($message:expr) => {
                CommandParserResult {
                    parsed_nodes,
                    error: Some(CommandParserError {
                        message: $message,
                        command,
                        index,
                    }),
                }
            };
        }

        let parsed_node = match spec.parse(name, command, index) {
            Ok(parsed_node) => parsed_node,
            Err(message) => return Err!(message),
        };
        index += parsed_node.len();
        parsed_nodes.push(parsed_node);

        if index >= command.len() {
            if spec.executable() {
                return Ok!();
            } else {
                return Err!("Incomplete command".to_string());
            }
        }

        const SPACE: char = ' ';
        if !command[index..].starts_with(SPACE) {
            return Err!(
                "Expected whitespace to end one argument, but found trailing data".to_string()
            );
        }
        index += SPACE.len_utf8();

        // let mut children = spec.children();
        let redirect = match spec.redirect() {
            Ok(ok) => ok,
            Err(message) => return Err!(message),
        };
        let children = if let Some(redirect) = redirect {
            if let Some(redirected) = self.specs.get(redirect) {
                parsed_nodes.push(ParsedNode::Redirect(redirect));
                redirected.children()
            } else {
                return Err!(format!("Failed to resolve redirect {}", redirect));
            }
        } else if spec.has_children() {
            spec.children()
        } else if !spec.executable() {
            // Special case for "execute run" which has no redirect to root for some reason
            &self.specs
        } else {
            return Err!("Incorrect argument for command".to_string());
        };
        let mut result = self.parse_from_specs(command, index, children);
        parsed_nodes.extend_from_slice(&result.parsed_nodes);
        result.parsed_nodes = parsed_nodes;
        result
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

impl ParsedNode<'_> {
    fn len(&self) -> usize {
        match self {
            ParsedNode::Redirect(_) => 0,
            ParsedNode::Literal { literal, .. } => literal.len(),
            ParsedNode::Argument { len, .. } => *len,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type", rename = "root")]
struct RootNode {
    children: BTreeMap<String, CommandSpec>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandSpec {
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

impl CommandSpec {
    fn parse<'l>(
        &self,
        name: &'l str,
        command: &'l str,
        index: usize,
    ) -> Result<ParsedNode<'l>, String> {
        let string = &command[index..];
        match self {
            CommandSpec::Literal { .. } => {
                let literal_len = string.find(' ').unwrap_or(string.len());
                let literal = &string[..literal_len];
                if literal == name {
                    Ok(ParsedNode::Literal { literal, index })
                } else {
                    Err("Incorrect literal for command".to_string())
                }
            }
            CommandSpec::Argument { parser, .. } => {
                parser
                    .parse(string)
                    .map(|(argument, len)| ParsedNode::Argument {
                        name,
                        argument,
                        index,
                        len,
                    })
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Node {
    #[serde(default)]
    pub children: BTreeMap<String, CommandSpec>,
    #[serde(default)]
    pub executable: bool,
    #[serde(default)]
    pub redirect: Vec<String>,
}

impl CommandSpec {
    pub fn has_children(&self) -> bool {
        !self.children().is_empty()
    }

    pub fn children(&self) -> &BTreeMap<String, CommandSpec> {
        match self {
            CommandSpec::Literal { node, .. } => &node.children,
            CommandSpec::Argument { node, .. } => &node.children,
        }
    }

    pub fn executable(&self) -> bool {
        match self {
            CommandSpec::Literal { node, .. } => node.executable,
            CommandSpec::Argument { node, .. } => node.executable,
        }
    }

    pub fn redirect(&self) -> Result<Option<&String>, String> {
        let redirect = match self {
            CommandSpec::Literal { node, .. } => &node.redirect,
            CommandSpec::Argument { node, .. } => &node.redirect,
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
        let actual = &CommandParser::default().unwrap().specs;

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
            children: BTreeMap::new(),
        };

        let actual = serde_json::to_string(&root).unwrap();

        // then:
        assert_eq!(actual, r#"{"type":"root","children":{}}"#);
    }
}
