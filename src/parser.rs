use crate::utils::split_once;
use const_format::concatcp;
use std::usize;

#[derive(Debug, PartialEq)]
pub enum Anchor {
    EYES,
    FEET,
}

#[derive(Debug, PartialEq)]
pub enum Line {
    Breakpoint,
    FunctionCall { name: String, anchor: Anchor },
    OtherCommand,
}

pub fn parse_line(line: &str) -> Line {
    let line = line.trim();
    if line == "# breakpoint" {
        Line::Breakpoint
    } else {
        if let Some(mut command) = parse_command(line) {
            let mut anchor = Anchor::FEET;
            loop {
                match command {
                    Command::Execute {
                        command: execute_command,
                    } => {
                        if let Some(a) = execute_command.anchor {
                            anchor = a;
                        }
                        command = *execute_command.run_command;
                    }
                    Command::Function { command } => {
                        break Line::FunctionCall {
                            name: command.function.to_string(),
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
    function: &'l str,
}

fn parse_command(string: &str) -> Option<Command> {
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

fn parse_function(rest: &str) -> Option<FunctionCommand> {
    let (function, _) = FunctionArgumentParser::parse(rest)?;
    Some(FunctionCommand {
        function: &function,
    })
}

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
    fn parse(string: &str) -> Option<((), &str)> {
        let string = string.strip_prefix('@')?;
        let string = string.strip_prefix(&['a', 'e', 'r', 's'][..])?;
        let string = string.strip_prefix('[')?;
        let end = 1 + string.find(']')?;
        Some(((), &string[end..]))
    }
}

struct FunctionArgumentParser;

const NAMESPACE_CHARS: &str = "0123456789abcdefghijklmnopqrstuvwxyz_-.";
const NAME_CHARS: &str = concatcp!(NAMESPACE_CHARS, "/");

impl<'l> ArgumentParser<'l, &'l str> for FunctionArgumentParser {
    fn parse(string: &'l str) -> Option<(&'l str, &'l str)> {
        let namespace_end = string.find(|c| !NAMESPACE_CHARS.contains(c))?;
        let (_namespace, rest) = string.split_at(namespace_end);
        let name = rest.strip_prefix(':')?;
        let name_end = name.find(|c| !NAME_CHARS.contains(c)).unwrap_or(name.len());
        let len = namespace_end + 1 + name_end;
        Some(string.split_at(len))
    }
}

struct SwizzleArgumentParser;

impl ArgumentParser<'_, ()> for SwizzleArgumentParser {
    fn parse(string: &str) -> Option<((), &str)> {
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
                name: "test:func".to_string(),
                anchor: Anchor::FEET,
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
                name: "test:func".to_string(),
                anchor: Anchor::FEET,
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
                name: "test:func".to_string(),
                anchor: Anchor::EYES,
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
                name: "test:func".to_string(),
                anchor: Anchor::EYES,
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
                name: "test:func".to_string(),
                anchor: Anchor::FEET,
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
                name: "test:func".to_string(),
                anchor: Anchor::FEET,
            }
        );
    }
}
