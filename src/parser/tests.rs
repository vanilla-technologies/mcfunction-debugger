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
                operation: ScheduleOperation::REPLACE {
                    time: MinecraftTime {
                        time: 1f32,
                        unit: MinecraftTimeUnit::Tick
                    }
                },
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
                operation: ScheduleOperation::APPEND {
                    time: MinecraftTime {
                        time: 1f32,
                        unit: MinecraftTimeUnit::Tick
                    }
                },
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
                operation: ScheduleOperation::CLEAR,
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
                operation: ScheduleOperation::REPLACE {
                    time: MinecraftTime {
                        time: 1f32,
                        unit: MinecraftTimeUnit::Tick
                    }
                },
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
                operation: ScheduleOperation::REPLACE {
                    time: MinecraftTime {
                        time: 1f32,
                        unit: MinecraftTimeUnit::Tick
                    }
                },
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
                operation: ScheduleOperation::CLEAR,
                selectors: vec![11],
            },
            None
        )
    );
}
