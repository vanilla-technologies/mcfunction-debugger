mod commands;

use self::commands::{default_commands, CommandsNode, Node};
use crate::utils::split_once;
use const_format::concatcp;
use std::{collections::HashMap, fmt::Display, usize};

#[derive(Debug, PartialEq)]
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
        if let Some(mut command) = parse_command(line) {
            let mut anchor = None;
            loop {
                match command {
                    Command::Execute {
                        command: execute_command,
                    } => {
                        anchor = execute_command.anchor.or(anchor);
                        command = *execute_command.run_command;
                    }
                    Command::Function { command } => {
                        break Line::FunctionCall {
                            name: command.function.to_owned(),
                            anchor,
                        }
                    }
                }
            }
        } else {
            Line::OtherCommand
        }
    }
}

enum Command<'l> {
    Execute { command: ExecuteCommand<'l> },
    Function { command: FunctionCommand<'l> },
}

struct ExecuteCommand<'l> {
    anchor: Option<Anchor>,
    run_command: Box<Command<'l>>,
}

struct FunctionCommand<'l> {
    function: NamespacedNameRef<&'l str>,
}

struct ParsedNode<'l> {
    value: ParsedNodeValue<'l>,
    child: Option<Box<ParsedNode<'l>>>,
}

enum ParsedNodeValue<'l> {
    Literal(&'l str),
    Swizzle(Swizzle),
}

fn parse_command(string: &str) -> Option<Command> {
    let commands = default_commands().ok()?;

    let visitor = CommandVisitor::new();
    visit_command(string, &commands, &visitor);

    let node: ParsedNode;

    match node.value {
        ParsedNodeValue::Literal("execute") => {
            if let Some(node) = node.child {
                match node.value {
                    ParsedNodeValue::Literal("anchored") => {
                        if let Some(node) = node.child {
                            match node.value {
                                ParsedNodeValue::Swizzle(_) => {}
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }

    let (command, rest) = split_once(string, ' ')?;
    match command {
        "execute" => {
            let command = parse_execute(rest)?;
            Some(Command::Execute { command })
        }
        "function" => {
            let command = parse_function(rest)?;
            Some(Command::Function { command })
        }
        _ => None,
    }
}

struct CommandVisitor {
    state: State,
}

enum State {
    None,
    Execute,
    Other,
}

impl Visitor for State {}

impl CommandVisitor {
    fn new() -> CommandVisitor {
        CommandVisitor { state: State::None }
    }
}

impl Visitor for CommandVisitor {
    fn visit_literal(&self, literal: &str) {
        match self.state {
            State::None => {
                if literal == "execute" {
                    self.state = State::Execute;
                } else {
                    self.state = State::Other;
                }
            }
            State::Execute => {}
            State::Other => {}
        }
    }
}

type Swizzle = ();

trait Visitor {
    fn visit_literal(&self, literal: &str) {}
    fn visit_swizzle(&self, swizzle: Swizzle) {}
}

fn visit_command(string: &str, commands: &HashMap<String, CommandsNode>, visitor: &dyn Visitor) {
    for (name, node) in commands {
        if let Some(suffix) = visit_node(node, name, string, visitor) {
            if suffix == "" && !node.executable() {
                // WARN
                println!("WARN");
            } else {
                if let Some(suffix) = suffix.strip_prefix(' ') {
                    visit_command(suffix, node.children(), visitor);
                } else {
                    // WARN
                    println!("WARN");
                }
            }
        }
    }
}

fn visit_node<'l>(
    node: &CommandsNode,
    name: &str,
    string: &'l str,
    visitor: &dyn Visitor,
) -> Option<&'l str> {
    match node {
        CommandsNode::Literal { node } => {
            let suffix = string.strip_prefix(name)?;
            visitor.visit_literal(name);
            Some(suffix)
        }
        CommandsNode::Argument { node, parser } => match parser {
            commands::ArgumentParser::MinecraftSwizzle => {
                let (swizzle, suffix) = SwizzleArgumentParser::parse(string)?;
                visitor.visit_swizzle(swizzle);
                Some(suffix)
            }
            commands::ArgumentParser::Unknown => None,
        },
    }
}

fn parse_function(rest: &str) -> Option<FunctionCommand> {
    let (function, _) = FunctionArgumentParser::parse(rest)?;
    Some(FunctionCommand { function })
}

fn parse_literal(node: &Node, string: &str) -> Option<ExecuteCommand> {}

fn parse_execute(string: &str) -> Option<ExecuteCommand> {
    let (child, rest) = split_once(string, ' ')?;
    match child {
        "align" => {
            let ((), rest) = SwizzleArgumentParser::parse(rest)?;
            let rest = rest.strip_prefix(' ')?;
            parse_execute(rest)
        }
        "anchored" => {
            let (anchor, rest) = EntityAnchorArgumentParser::parse(rest)?;
            parse_execute(rest).map(|mut command| {
                command.anchor = command.anchor.or(Some(anchor));
                command
            })
        }
        "as" => {
            let ((), rest) = EntityArgumentParser::parse(rest)?;
            let rest = rest.strip_prefix(' ')?;
            parse_execute(rest)
        }
        "at" => {
            let ((), rest) = EntityArgumentParser::parse(rest)?;
            let rest = rest.strip_prefix(' ')?;
            parse_execute(rest)
        }
        "run" => parse_command(rest).map(|command| ExecuteCommand {
            anchor: None,
            run_command: Box::new(command),
        }),
        _ => None,
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
            Some((Anchor::EYES, &string[eyes.len() + 1..]))
        } else if string.starts_with(feet) {
            Some((Anchor::FEET, &string[feet.len() + 1..]))
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
