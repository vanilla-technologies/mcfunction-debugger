pub mod commands;

use self::commands::{
    Argument, CommandParser, MinecraftEntityAnchor, MinecraftTime, NamespacedName, ParsedNode,
};

#[derive(Debug, PartialEq)]
pub enum Line {
    Breakpoint,
    FunctionCall {
        name: NamespacedName,
        anchor: Option<MinecraftEntityAnchor>,
        execute_as: bool,
    },
    Schedule {
        // TODO clear
        function: NamespacedName,
        time: MinecraftTime,
        append: bool,
    },
    OtherCommand,
}

pub fn parse_line(parser: &CommandParser, line: &str) -> Line {
    let line = line.trim();
    if line == "# breakpoint" {
        Line::Breakpoint
    } else {
        parse_command(parser, line).unwrap_or(Line::OtherCommand)
    }
}

fn parse_command(parser: &CommandParser, string: &str) -> Option<Line> {
    // TODO error handling
    let vec = parser.parse(string).ok()?;
    let mut nodes = vec.as_slice();

    let mut maybe_anchor: Option<MinecraftEntityAnchor> = None;
    let mut maybe_function = None;
    let mut execute_as = false;

    while let Some((head, tail)) = nodes.split_first() {
        nodes = tail;
        match head {
            ParsedNode::Literal("execute") | ParsedNode::Redirect("execute") => {
                if let Some((ParsedNode::Literal("anchored"), tail)) = tail.split_first() {
                    if let Some(ParsedNode::Argument(Argument::MinecraftEntityAnchor(anchor))) =
                        tail.first()
                    {
                        maybe_anchor = Some(*anchor);
                    }
                }
                if let Some((ParsedNode::Literal("as"), _tail)) = tail.split_first() {
                    execute_as = true;
                }
            }
            ParsedNode::Literal("function") => {
                if let Some(ParsedNode::Argument(Argument::MinecraftFunction(function))) =
                    tail.first()
                {
                    maybe_function = Some(function);
                }
            }
            ParsedNode::Literal("schedule") => {
                if let Some((ParsedNode::Literal("function"), tail)) = tail.split_first() {
                    if let Some((
                        ParsedNode::Argument(Argument::MinecraftFunction(function)),
                        tail,
                    )) = tail.split_first()
                    {
                        if let Some((ParsedNode::Argument(Argument::MinecraftTime(time)), tail)) =
                            tail.split_first()
                        {
                            return Some(Line::Schedule {
                                function: function.to_owned(),
                                time: time.clone(),
                                append: matches!(tail.first(), Some(ParsedNode::Literal("append"))),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let function = maybe_function?;
    Some(Line::FunctionCall {
        name: function.to_owned(),
        anchor: maybe_anchor,
        execute_as,
    })
}

#[cfg(test)]
mod tests {
    use crate::parser::commands::MinecraftTimeUnit;

    use super::*;

    #[test]
    fn test_breakpoint() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "# breakpoint";

        // when:
        let actual = parse_line(&parser, line);

        // then:
        assert_eq!(actual, Line::Breakpoint);
    }

    #[test]
    fn test_say() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "say execute run function test:func";

        // when:
        let actual = parse_line(&parser, line);

        // then:
        assert_eq!(actual, Line::OtherCommand);
    }

    #[test]
    fn test_execute() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute run function test:func";

        // when:
        let actual = parse_line(&parser, line);

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
        let actual = parse_line(&parser, line);

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
        let actual = parse_line(&parser, line);

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
        let actual = parse_line(&parser, line);

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
        let actual = parse_line(&parser, line);

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
        let actual = parse_line(&parser, line);

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
        let actual = parse_line(&parser, line);

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
        let actual = parse_line(&parser, line);

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
        let actual = parse_line(&parser, line);

        // then:
        assert_eq!(
            actual,
            Line::Schedule {
                function: NamespacedName::from("test:func".to_owned()).unwrap(),
                time: MinecraftTime {
                    time: 1f32,
                    unit: MinecraftTimeUnit::Tick
                },
                append: false,
            }
        );
    }

    #[test]
    fn test_schedule_replace() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "schedule function test:func 1t replace";

        // when:
        let actual = parse_line(&parser, line);

        // then:
        assert_eq!(
            actual,
            Line::Schedule {
                function: NamespacedName::from("test:func".to_owned()).unwrap(),
                time: MinecraftTime {
                    time: 1f32,
                    unit: MinecraftTimeUnit::Tick
                },
                append: false,
            }
        );
    }

    #[test]
    fn test_schedule_append() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "schedule function test:func 1t append";

        // when:
        let actual = parse_line(&parser, line);

        // then:
        assert_eq!(
            actual,
            Line::Schedule {
                function: NamespacedName::from("test:func".to_owned()).unwrap(),
                time: MinecraftTime {
                    time: 1f32,
                    unit: MinecraftTimeUnit::Tick
                },
                append: true,
            }
        );
    }

    #[test]
    fn test_execute_schedule() {
        // given:
        let parser = CommandParser::default().unwrap();
        let line = "execute at @e[type=area_effect_cloud] run schedule function test:func 1t";

        // when:
        let actual = parse_line(&parser, line);

        // then:
        assert_eq!(
            actual,
            Line::Schedule {
                function: NamespacedName::from("test:func".to_owned()).unwrap(),
                time: MinecraftTime {
                    time: 1f32,
                    unit: MinecraftTimeUnit::Tick
                },
                append: false,
            }
        );
    }
}
