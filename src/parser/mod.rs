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
mod tests {
    use super::*;
    use crate::parser::commands::MinecraftTimeUnit;

    #[test]
    fn test_breakpoint() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "# breakpoint";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(actual, (Line::Breakpoint, None));
    }

    #[test]
    fn test_say() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "say execute as @e run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::OtherCommand {
                    selectors: vec![15]
                },
                None
            )
        );
    }

    #[test]
    fn test_tellraw() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = r#"tellraw @a {"text":"Hello World!"}"#;

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        // TODO support argument type: minecraft:component
        assert_eq!(actual.0, Line::OtherCommand { selectors: vec![8] });
    }

    #[test]
    fn test_scoreboard_operation_selectors() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "scoreboard players operation @s test = @e[tag=test] test";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::OtherCommand {
                    selectors: vec![29, 39],
                },
                None
            )
        );
    }

    #[test]
    fn test_scoreboard_operation_names() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "scoreboard players operation var1 test = var2 test";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::OtherCommand {
                    selectors: Vec::new(),
                },
                None
            )
        );
    }

    #[test]
    fn test_execute() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::FunctionCall {
                    name: NamespacedName::from("test:func".to_owned()).unwrap(),
                    anchor: None,
                    execute_as: false,
                    selectors: Vec::new(),
                },
                None
            )
        );
    }

    #[test]
    fn test_execute_align() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute align xyz run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::FunctionCall {
                    name: NamespacedName::from("test:func".to_owned()).unwrap(),
                    anchor: None,
                    execute_as: false,
                    selectors: Vec::new(),
                },
                None
            )
        );
    }

    #[test]
    fn test_execute_anchored() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute anchored eyes run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::FunctionCall {
                    name: NamespacedName::from("test:func".to_owned()).unwrap(),
                    anchor: Some(MinecraftEntityAnchor::EYES),
                    execute_as: false,
                    selectors: Vec::new(),
                },
                None
            )
        );
    }

    #[test]
    fn test_execute_multiple_anchored() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute anchored feet anchored eyes run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::FunctionCall {
                    name: NamespacedName::from("test:func".to_owned()).unwrap(),
                    anchor: Some(MinecraftEntityAnchor::EYES),
                    execute_as: false,
                    selectors: Vec::new(),
                },
                None
            )
        );
    }

    #[test]
    fn test_multiple_execute_anchored() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute anchored feet run execute anchored eyes run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::FunctionCall {
                    name: NamespacedName::from("test:func".to_owned()).unwrap(),
                    anchor: Some(MinecraftEntityAnchor::EYES),
                    execute_as: false,
                    selectors: Vec::new(),
                },
                None
            )
        );
    }

    #[test]
    fn test_multiple_execute_some_anchored() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute anchored eyes run execute as @s run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::FunctionCall {
                    name: NamespacedName::from("test:func".to_owned()).unwrap(),
                    anchor: Some(MinecraftEntityAnchor::EYES),
                    execute_as: true,
                    selectors: vec![37],
                },
                None
            )
        );
    }

    #[test]
    fn test_execute_as() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute as @e[type=area_effect_cloud] run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::FunctionCall {
                    name: NamespacedName::from("test:func".to_owned()).unwrap(),
                    anchor: None,
                    execute_as: true,
                    selectors: vec![11],
                },
                None
            )
        );
    }

    #[test]
    fn test_execute_at() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute at @e[type=area_effect_cloud] run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::FunctionCall {
                    name: NamespacedName::from("test:func".to_owned()).unwrap(),
                    anchor: None,
                    execute_as: false,
                    selectors: vec![11],
                },
                None
            )
        );
    }

    #[test]
    fn test_execute_positioned_absolute() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute positioned -1 0 1 run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::FunctionCall {
                    name: NamespacedName::from("test:func".to_owned()).unwrap(),
                    anchor: None,
                    execute_as: false,
                    selectors: Vec::new(),
                },
                None
            )
        );
    }

    #[test]
    fn test_execute_positioned_local() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute positioned ^-1 ^.3 ^-4.5 run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::FunctionCall {
                    name: NamespacedName::from("test:func".to_owned()).unwrap(),
                    anchor: None,
                    execute_as: false,
                    selectors: Vec::new(),
                },
                None
            )
        );
    }

    #[test]
    fn test_execute_positioned_relative() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute positioned ~-1 ~.3 ~-4.5 run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::FunctionCall {
                    name: NamespacedName::from("test:func".to_owned()).unwrap(),
                    anchor: None,
                    execute_as: false,
                    selectors: Vec::new(),
                },
                None
            )
        );
    }

    #[test]
    fn test_execute_rotated_absolute() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute rotated 1 -5 run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::FunctionCall {
                    name: NamespacedName::from("test:func".to_owned()).unwrap(),
                    anchor: None,
                    execute_as: false,
                    selectors: Vec::new(),
                },
                None
            )
        );
    }

    #[test]
    fn test_execute_rotated_relative() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute rotated ~ ~-.5 run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::FunctionCall {
                    name: NamespacedName::from("test:func".to_owned()).unwrap(),
                    anchor: None,
                    execute_as: false,
                    selectors: Vec::new(),
                },
                None
            )
        );
    }

    #[test]
    fn test_schedule() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "schedule function test:func 1t";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::Schedule {
                    schedule_start: 0,
                    function: NamespacedName::from("test:func".to_owned()).unwrap(),
                    time: Some(MinecraftTime {
                        time: 1f32,
                        unit: MinecraftTimeUnit::Tick
                    }),
                    category: None,
                    selectors: Vec::new(),
                },
                None
            )
        );
    }

    #[test]
    fn test_schedule_append() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "schedule function test:func 1t append";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::Schedule {
                    schedule_start: 0,
                    function: NamespacedName::from("test:func".to_owned()).unwrap(),
                    time: Some(MinecraftTime {
                        time: 1f32,
                        unit: MinecraftTimeUnit::Tick
                    }),
                    category: Some("append".to_string()),
                    selectors: Vec::new(),
                },
                None
            )
        );
    }

    #[test]
    fn test_schedule_clear() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "schedule clear test:func";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::Schedule {
                    schedule_start: 0,
                    function: NamespacedName::from("test:func".to_owned()).unwrap(),
                    time: None,
                    category: Some("clear".to_string()),
                    selectors: Vec::new(),
                },
                None
            )
        );
    }

    #[test]
    fn test_schedule_replace() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "schedule function test:func 1t replace";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::Schedule {
                    schedule_start: 0,
                    function: NamespacedName::from("test:func".to_owned()).unwrap(),
                    time: Some(MinecraftTime {
                        time: 1f32,
                        unit: MinecraftTimeUnit::Tick
                    }),
                    category: Some("replace".to_string()),
                    selectors: Vec::new(),
                },
                None
            )
        );
    }

    #[test]
    fn test_execute_schedule() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute at @e[type=area_effect_cloud] run schedule function test:func 1t";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::Schedule {
                    schedule_start: 42,
                    function: NamespacedName::from("test:func".to_owned()).unwrap(),
                    time: Some(MinecraftTime {
                        time: 1f32,
                        unit: MinecraftTimeUnit::Tick
                    }),
                    category: None,
                    selectors: vec![11],
                },
                None
            )
        );
    }

    #[test]
    fn test_execute_schedule_clear() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute at @e[type=area_effect_cloud] run schedule clear test:func";

        // when:
        let actual = parse_line_internal(&parser, line);

        // then:
        assert_eq!(
            actual,
            (
                Line::Schedule {
                    schedule_start: 42,
                    function: NamespacedName::from("test:func".to_owned()).unwrap(),
                    time: None,
                    category: Some("clear".to_string()),
                    selectors: vec![11],
                },
                None
            )
        );
    }
}
