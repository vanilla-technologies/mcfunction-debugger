pub mod commands;

use self::commands::{default_commands, Argument, CommandsNode, EntityAnchor, NamespacedName};
use log::warn;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, PartialEq)]
pub enum Line {
    Breakpoint,
    FunctionCall {
        name: NamespacedName,
        anchor: Option<EntityAnchor>,
    },
    OtherCommand,
}

pub fn parse_line(line: &str) -> Line {
    let line = line.trim();
    if line == "# breakpoint" {
        Line::Breakpoint
    } else {
        parse_function_call(line).unwrap_or(Line::OtherCommand)
    }
}

fn parse_function_call(string: &str) -> Option<Line> {
    let commands = default_commands().ok()?;

    // TODO error handling
    let vec = Vec::from(parse_command(string, &commands, &commands).ok()?);
    let mut nodes = vec.as_slice();

    let mut maybe_anchor: Option<EntityAnchor> = None;
    let mut maybe_function = None;

    while let Some((head, tail)) = nodes.split_first() {
        nodes = tail;
        match head {
            ParsedNode::Literal("execute") | ParsedNode::Redirect("execute") => {
                if let Some((ParsedNode::Literal("anchored"), tail)) = tail.split_first() {
                    if let Some(ParsedNode::Argument(Argument::EntityAnchor(anchor))) = tail.first()
                    {
                        maybe_anchor = Some(*anchor);
                    }
                }
            }
            ParsedNode::Literal("function") => {
                if let Some(ParsedNode::Argument(Argument::Function(function))) = tail.first() {
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

fn parse_command<'l>(
    string: &'l str,
    commands: &'l HashMap<String, CommandsNode>,
    sub_commands: &'l HashMap<String, CommandsNode>,
) -> Result<VecDeque<ParsedNode<'l>>, String> {
    for (node_name, command_node) in sub_commands {
        if let Some((parsed_node, suffix)) = parse_node(string, command_node, node_name) {
            return parse_suffix(suffix, command_node, parsed_node, commands);
        }
    }
    // TODO error handling
    Err("Unknown command, see below for error\nabcd<--[HERE]".to_string())
}

enum ParsedNode<'l> {
    Redirect(&'l str),
    Literal(&'l str),
    Argument(Argument<'l>),
}

fn parse_node<'l>(
    string: &'l str,
    node: &CommandsNode,
    node_name: &str,
) -> Option<(ParsedNode<'l>, &'l str)> {
    match node {
        CommandsNode::Literal { .. } => {
            let end = string.find(' ').unwrap_or(string.len());
            let (literal, suffix) = string.split_at(end);
            if literal == node_name {
                Some((ParsedNode::Literal(literal), suffix))
            } else {
                None
            }
        }
        CommandsNode::Argument { parser, .. } => {
            let (argument, suffix) = parser
                .parse(string)
                .map_err(|e| warn!("Failed to parse argument {} due to: {}", node_name, e))
                .ok()?;
            Some((ParsedNode::Argument(argument), suffix))
        }
    }
}

fn parse_suffix<'l>(
    suffix: &'l str,
    command_node: &'l CommandsNode,
    parsed_node: ParsedNode<'l>,
    commands: &'l HashMap<String, CommandsNode>,
) -> Result<VecDeque<ParsedNode<'l>>, String> {
    if suffix == "" {
        if command_node.executable() {
            Ok(VecDeque::from(vec![parsed_node]))
        } else {
            // TODO error handling
            Err("Incomplete command, see below for error\n...hored eyes<--[HERE]".to_string())
        }
    } else {
        let mut redirect_node = None;
        // let children = command_node.children(commands)?;
        let children = command_node.children();
        let children = if !children.is_empty() {
            Ok(children)
        } else {
            if let Some(redirect) = command_node.redirect()? {
                let command = commands
                    .get(redirect)
                    .ok_or(format!("Failed to resolve redirect {}", redirect))?;
                redirect_node = Some(ParsedNode::Redirect(redirect));
                Ok(command.children())
            } else if !command_node.executable() {
                // Special case for run which has no redirect to root for some reason
                Ok(commands)
            } else {
                // TODO error handling
                Err("Expected whitespace to end one argument, but found trailing data at position 22: ...hored eyes#<--[HERE]".to_string())
            }
        }?;

        if children.is_empty() {
            // TODO error handling
            Err("Incorrect argument for command at position 13: ...me set day<--[HERE]".to_string())
        } else {
            let suffix = suffix.strip_prefix(' ').ok_or(
                "Expected whitespace to end one argument, but found trailing data at position 22: ...hored eyes#<--[HERE]".to_string(),
            )?;
            let mut nodes = parse_command(suffix, commands, children)?;
            if let Some(redirect_node) = redirect_node {
                nodes.push_front(redirect_node);
            }
            nodes.push_front(parsed_node);
            Ok(nodes)
        }
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
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
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
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
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
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: Some(EntityAnchor::EYES),
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
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: Some(EntityAnchor::EYES),
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
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: Some(EntityAnchor::EYES),
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
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: Some(EntityAnchor::EYES),
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
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
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
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: None,
            }
        );
    }
}
