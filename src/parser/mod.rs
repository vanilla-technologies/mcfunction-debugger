pub mod commands;

use self::commands::{
    Argument, CommandParser, CommandParserError, MinecraftEntityAnchor, MinecraftScoreHolder,
    MinecraftTime, NamespacedName, NamespacedNameRef, ParsedNode,
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
    },
    Schedule {
        schedule_start: usize,
        function: NamespacedName,
        time: Option<MinecraftTime>,
        category: Option<String>,
    },
    OtherCommand {
        selectors: Vec<usize>,
    },
}

pub fn parse_line(parser: &CommandParser, line: &str) -> Line {
    parse_line_internal(parser, line)
        .map_err(|e| warn!("Failed to parse command {} due to: {}", line, e))
        .unwrap_or(Line::OtherCommand {
            selectors: Vec::new(),
        })
}

fn parse_line_internal<'l>(
    parser: &'l CommandParser,
    line: &'l str,
) -> Result<Line, CommandParserError<'l>> {
    let line = line.trim();
    if line == "# breakpoint" {
        Ok(Line::Breakpoint)
    } else {
        parse_command(parser, line)
    }
}

fn parse_command<'l>(
    parser: &'l CommandParser,
    command: &'l str,
) -> Result<Line, CommandParserError<'l>> {
    let vec = parser.parse(command)?;
    let mut nodes = vec.as_slice();
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
                    return Ok(Line::FunctionCall {
                        name: function.to_owned(),
                        anchor: maybe_anchor,
                        execute_as,
                    });
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
                            return Ok(Line::Schedule {
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
                            });
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
                            return Ok(Line::Schedule {
                                schedule_start: *index,
                                function: function.to_owned(),
                                time: None,
                                category: Some("clear".to_string()),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(Line::OtherCommand { selectors })
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
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(actual, Line::Breakpoint);
    }

    #[test]
    fn test_say() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "say execute run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::OtherCommand {
                selectors: Vec::new()
            }
        );
    }

    #[test]
    fn test_tellraw() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = r#"tellraw @a {"text":"HelloWorld!"}"#;

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(actual, Line::OtherCommand { selectors: vec![8] });
    }

    #[test]
    fn test_scoreboard_operation_selectors() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "scoreboard players operation @s test = @e[tag=test] test";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::OtherCommand {
                selectors: vec![29, 39]
            }
        );
    }

    #[test]
    fn test_scoreboard_operation_names() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "scoreboard players operation var1 test = var2 test";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::OtherCommand {
                selectors: Vec::new()
            }
        );
    }

    #[test]
    fn test_execute() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: None,
                execute_as: false
            }
        );
    }

    #[test]
    fn test_execute_align() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute align xyz run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: None,
                execute_as: false
            }
        );
    }

    #[test]
    fn test_execute_anchored() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute anchored eyes run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: Some(MinecraftEntityAnchor::EYES),
                execute_as: false
            }
        );
    }

    #[test]
    fn test_execute_multiple_anchored() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute anchored feet anchored eyes run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: Some(MinecraftEntityAnchor::EYES),
                execute_as: false
            }
        );
    }

    #[test]
    fn test_multiple_execute_anchored() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute anchored feet run execute anchored eyes run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: Some(MinecraftEntityAnchor::EYES),
                execute_as: false
            }
        );
    }

    #[test]
    fn test_multiple_execute_some_anchored() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute anchored eyes run execute as @s run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: Some(MinecraftEntityAnchor::EYES),
                execute_as: true
            }
        );
    }

    #[test]
    fn test_execute_as() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute as @e[type=area_effect_cloud] run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: None,
                execute_as: true,
            }
        );
    }

    #[test]
    fn test_execute_at() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute at @e[type=area_effect_cloud] run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: None,
                execute_as: false
            }
        );
    }

    #[test]
    fn test_execute_positioned_absolute() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute positioned -1 0 1 run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: None,
                execute_as: false
            }
        );
    }

    #[test]
    fn test_execute_positioned_local() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute positioned ^-1 ^.3 ^-4.5 run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: None,
                execute_as: false
            }
        );
    }

    #[test]
    fn test_execute_positioned_relative() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute positioned ~-1 ~.3 ~-4.5 run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: None,
                execute_as: false
            }
        );
    }

    #[test]
    fn test_execute_rotated_absolute() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute rotated 1 -5 run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: None,
                execute_as: false
            }
        );
    }

    #[test]
    fn test_execute_rotated_relative() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute rotated ~ ~-.5 run function test:func";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::FunctionCall {
                name: NamespacedName::from("test:func".to_owned()).unwrap(),
                anchor: None,
                execute_as: false
            }
        );
    }

    #[test]
    fn test_schedule() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "schedule function test:func 1t";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::Schedule {
                schedule_start: 0,
                function: NamespacedName::from("test:func".to_owned()).unwrap(),
                time: Some(MinecraftTime {
                    time: 1f32,
                    unit: MinecraftTimeUnit::Tick
                }),
                category: None,
            }
        );
    }

    #[test]
    fn test_schedule_append() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "schedule function test:func 1t append";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::Schedule {
                schedule_start: 0,
                function: NamespacedName::from("test:func".to_owned()).unwrap(),
                time: Some(MinecraftTime {
                    time: 1f32,
                    unit: MinecraftTimeUnit::Tick
                }),
                category: Some("append".to_string()),
            }
        );
    }

    #[test]
    fn test_schedule_clear() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "schedule clear test:func";

        // when:
        let actual = parse_line(&parser, line);

        // then:
        assert_eq!(
            actual,
            Line::Schedule {
                schedule_start: 0,
                function: NamespacedName::from("test:func".to_owned()).unwrap(),
                time: None,
                category: Some("clear".to_string()),
            }
        );
    }

    #[test]
    fn test_schedule_replace() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "schedule function test:func 1t replace";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::Schedule {
                schedule_start: 0,
                function: NamespacedName::from("test:func".to_owned()).unwrap(),
                time: Some(MinecraftTime {
                    time: 1f32,
                    unit: MinecraftTimeUnit::Tick
                }),
                category: Some("replace".to_string()),
            }
        );
    }

    #[test]
    fn test_execute_schedule() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute at @e[type=area_effect_cloud] run schedule function test:func 1t";

        // when:
        let actual = parse_line_internal(&parser, line).unwrap();

        // then:
        assert_eq!(
            actual,
            Line::Schedule {
                schedule_start: 42,
                function: NamespacedName::from("test:func".to_owned()).unwrap(),
                time: Some(MinecraftTime {
                    time: 1f32,
                    unit: MinecraftTimeUnit::Tick
                }),
                category: None,
            }
        );
    }

    #[test]
    fn test_execute_schedule_clear() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute at @e[type=area_effect_cloud] run schedule clear test:func";

        // when:
        let actual = parse_line(&parser, line);

        // then:
        assert_eq!(
            actual,
            Line::Schedule {
                schedule_start: 42,
                function: NamespacedName::from("test:func".to_owned()).unwrap(),
                time: None,
                category: Some("clear".to_string()),
            }
        );
    }
}
