mod commands;

use self::commands::{default_commands, CommandsNode};
use const_format::concatcp;
use std::{
    collections::{HashMap, VecDeque},
    fmt::Display,
    usize,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Anchor {
    EYES,
    FEET,
}

#[derive(Debug, PartialEq)]
pub enum Line {
    Breakpoint,
    FunctionCall {
        name: NamespacedName,
        anchor: Option<Anchor>,
    },
    OtherCommand,
}

pub type NamespacedName = NamespacedNameRef<String>;

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct NamespacedNameRef<S: AsRef<str>> {
    string: S,
    namespace_len: usize,
}

impl<S: AsRef<str>> NamespacedNameRef<S> {
    pub fn new(string: S, namespace_len: usize) -> NamespacedNameRef<S> {
        NamespacedNameRef {
            string,
            namespace_len,
        }
    }

    pub fn namespace(&self) -> &str {
        &self.string.as_ref()[..self.namespace_len]
    }

    pub fn name(&self) -> &str {
        &self.string.as_ref()[self.namespace_len + 1..]
    }

    pub fn to_owned(&self) -> NamespacedName {
        NamespacedName {
            string: self.string.as_ref().to_owned(),
            namespace_len: self.namespace_len,
        }
    }
}

impl<S: AsRef<str>> Display for NamespacedNameRef<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.string.as_ref().fmt(f)
    }
}

pub fn parse_line(line: &str) -> Line {
    let line = line.trim();
    if line == "# breakpoint" {
        Line::Breakpoint
    } else {
        parse_command(line).unwrap_or(Line::OtherCommand)
    }
}

type Function<'l> = NamespacedNameRef<&'l str>;
type Swizzle = ();

enum ParsedNode<'l> {
    Redirect(&'l str),
    Literal(&'l str),
    Function(Function<'l>),
    Swizzle(Swizzle),
    Anchor(Anchor),
}

fn parse_command(string: &str) -> Option<Line> {
    let commands = default_commands().ok()?;

    // TODO avoid copying
    let mut sub_commands = HashMap::new();
    sub_commands.extend(commands.iter());

    // TODO error handling
    let vec = Vec::from(parse_command2(string, &commands, &sub_commands).ok()?);
    let mut nodes = vec.as_slice();

    let mut maybe_anchor: Option<Anchor> = None;
    let mut maybe_function = None;

    while let Some((head, tail)) = nodes.split_first() {
        nodes = tail;
        match head {
            ParsedNode::Literal("execute") | ParsedNode::Redirect("execute") => {
                if let Some((ParsedNode::Literal("anchored"), tail)) = tail.split_first() {
                    if let Some(ParsedNode::Anchor(anchor)) = tail.first() {
                        maybe_anchor = Some(*anchor);
                    }
                }
            }
            ParsedNode::Literal("function") => {
                if let Some(ParsedNode::Function(function)) = tail.first() {
                    maybe_function = Some(function);
                }
            }
            _ => {}
        }
    }

    let function = maybe_function?;
    Some(Line::FunctionCall {
        name: function.to_owned(),
        anchor: maybe_anchor,
    })
}

fn parse_command2<'l>(
    string: &'l str,
    commands: &HashMap<String, CommandsNode>,
    sub_commands: &HashMap<&String, &CommandsNode>,
) -> Result<VecDeque<ParsedNode<'l>>, String> {
    // println!("{:#?}", sub_commands);
    for (node_name, command_node) in sub_commands {
        println!("{}", node_name);
        if let Some((parsed_node, suffix)) = parse_node(string, command_node, node_name) {
            if suffix == "" {
                if !command_node.executable() {
                    // TODO error handling
                    return Err(
                    "Unknown or incomplete command, see below for error\n...hored eyes<--[HERE]"
                        .to_string(),
                );
                } else {
                    let mut result = VecDeque::new();
                    result.push_front(parsed_node);
                    return Ok(result);
                }
            } else {
                if let Some(suffix) = suffix.strip_prefix(' ') {
                    // TODO avoid copying
                    let mut sub_commands = HashMap::new();
                    let children = command_node.children();
                    if children.is_empty() {
                        if command_node.redirect().is_empty() && !command_node.executable() {
                            // Special case for run which has no redirect to root for some reason
                            sub_commands.extend(commands);
                        } else {
                            // TODO Add Redirect Node
                            for redirect in command_node.redirect() {
                                let command = commands
                                    .get(redirect)
                                    .ok_or(format!("Failed to resolve redirect {}", redirect))?;

                                sub_commands.extend(command.children());
                            }
                        }
                    } else {
                        sub_commands.extend(children);
                    }

                    let mut nodes = parse_command2(suffix, commands, &sub_commands)?;
                    nodes.push_front(parsed_node);
                    return Ok(nodes);
                } else {
                    // TODO error handling
                    return Err("Expected whitespace to end one argument, but found trailing data at position 22: ...hored eyes#<--[HERE]".to_string());
                }
            }
        }
    }
    // TODO error handling
    return Err("Unknown or incomplete command, see below for error\nabcd<--[HERE]".to_string());
}

fn parse_node<'l>(
    string: &'l str,
    node: &CommandsNode,
    node_name: &str,
) -> Option<(ParsedNode<'l>, &'l str)> {
    match node {
        CommandsNode::Literal { .. } => {
            if string.starts_with(node_name) {
                let (node_name, suffix) = string.split_at(node_name.len());
                Some((ParsedNode::Literal(node_name), suffix))
            } else {
                None
            }
        }
        CommandsNode::Argument { parser, .. } => match parser {
            // TODO refactor
            commands::ArgumentParser::MinecraftFunction => {
                let (function, suffix) = FunctionArgumentParser::parse(string)?;
                Some((ParsedNode::Function(function), suffix))
            }
            commands::ArgumentParser::MinecraftSwizzle => {
                let (swizzle, suffix) = SwizzleArgumentParser::parse(string)?;
                Some((ParsedNode::Swizzle(swizzle), suffix))
            }
            commands::ArgumentParser::MinecraftEntityAnchor => {
                let (anchor, suffix) = EntityAnchorArgumentParser::parse(string)?;
                Some((ParsedNode::Anchor(anchor), suffix))
            }
            commands::ArgumentParser::Unknown => None,
        },
    }
}

trait ArgumentParser<'s, A> {
    fn parse(string: &'s str) -> Option<(A, &'s str)>;
}

struct EntityAnchorArgumentParser;

impl ArgumentParser<'_, Anchor> for EntityAnchorArgumentParser {
    fn parse(string: &str) -> Option<(Anchor, &str)> {
        let eyes = "eyes";
        let feet = "feet";
        if string.starts_with(eyes) {
            Some((Anchor::EYES, &string[eyes.len()..]))
        } else if string.starts_with(feet) {
            Some((Anchor::FEET, &string[feet.len()..]))
        } else {
            None
        }
    }
}

struct EntityArgumentParser;

// TODO support ] in strings and NBT
// TODO support for player name and UUID
// TODO add support for limits on amount and type
impl ArgumentParser<'_, ()> for EntityArgumentParser {
    fn parse(mut string: &str) -> Option<((), &str)> {
        string = string.strip_prefix('@')?;
        string = string.strip_prefix(&['a', 'e', 'r', 's'][..])?;
        let end = if string.starts_with(' ') {
            0
        } else {
            string = string.strip_prefix('[')?;
            1 + string.find(']')?
        };
        Some(((), &string[end..]))
    }
}

struct FunctionArgumentParser;

const NAMESPACE_CHARS: &str = "0123456789abcdefghijklmnopqrstuvwxyz_-.";
const NAME_CHARS: &str = concatcp!(NAMESPACE_CHARS, "/");

impl<'l> ArgumentParser<'l, NamespacedNameRef<&'l str>> for FunctionArgumentParser {
    fn parse(string: &'l str) -> Option<(NamespacedNameRef<&'l str>, &'l str)> {
        let namespace_end = string.find(|c| !NAMESPACE_CHARS.contains(c))?;
        let (_namespace, rest) = string.split_at(namespace_end);
        let rest = rest.strip_prefix(':')?;
        let name_end = rest.find(|c| !NAME_CHARS.contains(c)).unwrap_or(rest.len());
        let len = namespace_end + 1 + name_end;
        let (string, rest) = string.split_at(len);
        let name = NamespacedNameRef {
            string,
            namespace_len: namespace_end,
        };
        Some((name, rest))
    }
}

struct SwizzleArgumentParser;

impl ArgumentParser<'_, ()> for SwizzleArgumentParser {
    fn parse(string: &str) -> Option<(Swizzle, &str)> {
        let end = string.find(' ')?;
        Some(((), &string[end..]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breakpoint() {
        // given:
        let line = "# breakpoint";

        // when:
        let actual = parse_line(line);

        // then:
        assert_eq!(actual, Line::Breakpoint);
    }

    #[test]
    fn test_say() {
        // given:
        let line = "say execute run function test:func";

        // when:
        let actual = parse_line(line);

        // then:
        assert_eq!(actual, Line::OtherCommand);
    }

    #[test]
    fn test_execute() {
        // given:
        let line = "execute run function test:func";

        // when:
        let actual = parse_line(line);

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName {
                    string: "test:func".to_string(),
                    namespace_len: 4,
                },
                anchor: None,
            }
        );
    }

    #[test]
    fn test_execute_align() {
        // given:
        let line = "execute align xyz run function test:func";

        // when:
        let actual = parse_line(line);

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName {
                    string: "test:func".to_string(),
                    namespace_len: 4,
                },
                anchor: None,
            }
        );
    }

    #[test]
    fn test_execute_anchored() {
        // given:
        let line = "execute anchored eyes run function test:func";

        // when:
        let actual = parse_line(line);

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName {
                    string: "test:func".to_string(),
                    namespace_len: 4,
                },
                anchor: Some(Anchor::EYES),
            }
        );
    }

    #[test]
    fn test_execute_multiple_anchored() {
        // given:
        let line = "execute anchored feet anchored eyes run function test:func";

        // when:
        let actual = parse_line(line);

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName {
                    string: "test:func".to_string(),
                    namespace_len: 4,
                },
                anchor: Some(Anchor::EYES),
            }
        );
    }

    #[test]
    fn test_multiple_execute_anchored() {
        // given:
        let line = "execute anchored feet run execute anchored eyes run function test:func";

        // when:
        let actual = parse_line(line);

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName {
                    string: "test:func".to_string(),
                    namespace_len: 4,
                },
                anchor: Some(Anchor::EYES),
            }
        );
    }

    #[test]
    fn test_multiple_execute_some_anchored() {
        // given:
        let line = "execute anchored eyes run execute as @s run function test:func";

        // when:
        let actual = parse_line(line);

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName {
                    string: "test:func".to_string(),
                    namespace_len: 4,
                },
                anchor: Some(Anchor::EYES),
            }
        );
    }

    #[test]
    fn test_execute_as() {
        // given:
        let line = "execute as @e[type=area_effect_cloud] run function test:func";

        // when:
        let actual = parse_line(line);

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName {
                    string: "test:func".to_string(),
                    namespace_len: 4,
                },
                anchor: None,
            }
        );
    }

    #[test]
    fn test_execute_at() {
        // given:
        let line = "execute at @e[type=area_effect_cloud] run function test:func";

        // when:
        let actual = parse_line(line);

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName {
                    string: "test:func".to_string(),
                    namespace_len: 4,
                },
                anchor: None,
            }
        );
    }
}
