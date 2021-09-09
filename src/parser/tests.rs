use super::*;
use crate::parser::command::MinecraftTimeUnit;

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
    assert_eq!(actual, (Line::OtherCommand { selectors: vec![] }, None));
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
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
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
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
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
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: Some(MinecraftEntityAnchor::EYES),
                execute_as: false,
                selectors: vec![],
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
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: Some(MinecraftEntityAnchor::EYES),
                execute_as: false,
                selectors: vec![],
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
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: Some(MinecraftEntityAnchor::EYES),
                execute_as: false,
                selectors: vec![],
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
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
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
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
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
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![11],
            },
            None
        )
    );
}

#[test]
fn test_execute_facing_pos() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = "execute facing 1 ~2 -3 run function test:func";

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
            },
            None
        )
    );
}

#[test]
fn test_execute_facing_entity() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = "execute facing entity @e[type=area_effect_cloud] eyes run function test:func";

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![22],
            },
            None
        )
    );
}

#[test]
fn test_execute_in() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = "execute in the_nether run function test:func";

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
            },
            None
        )
    );
}

#[test]
fn test_execute_in_qualified() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = "execute in minecraft:the_end run function test:func";

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
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
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
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
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
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
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
            },
            None
        )
    );
}

#[test]
fn test_execute_positioned_as() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = "execute positioned as @e[type=area_effect_cloud] run function test:func";

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![22],
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
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
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
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
            },
            None
        )
    );
}

#[test]
fn test_execute_rotated_as() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = "execute rotated as @e[type=area_effect_cloud] run function test:func";

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![19],
            },
            None
        )
    );
}

#[test]
fn test_execute_if_block() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = r#"execute if block ^1 ^.25 ^-.75 chest[facing=east]{Items:[{id:"minecraft:apple",Slot:13b,Count:1b}]} run function test:func"#;

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
            },
            None
        )
    );
}

#[test]
fn test_execute_if_block_tag() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line =
        r#"execute if block ^1 ^.25 ^-.75 #minecraft:stairs[facing=east] run function test:func"#;

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
            },
            None
        )
    );
}

#[test]
fn test_execute_if_blocks() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line =
        r#"execute if block -0 ~-.3 ~5 ^1 ^.25 ^-.75 ~-1 .5 -.75 all run function test:func"#;

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
            },
            None
        )
    );
}

#[test]
fn test_execute_if_data_block() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = r#"execute if data block 1 2 3 Items[{Slot:13b}] run function test:func"#;

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
            },
            None
        )
    );
}

#[test]
fn test_execute_if_data_entity() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = r#"execute if data entity @p Inventory[0].tag.BlockEntityTag.Command run function test:func"#;

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![23],
            },
            None
        )
    );
}

#[test]
fn test_execute_if_data_storage() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = r#"execute if data storage test foo.bar[0][0].baz run function test:func"#;

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
            },
            None
        )
    );
}

#[test]
fn test_execute_if_entity() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = r#"execute if entity @e[type=area_effect_cloud] run function test:func"#;

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![18],
            },
            None
        )
    );
}

#[test]
fn test_execute_if_predicate() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = r#"execute if predicate mcfd:test_pred run function test:func"#;

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
            },
            None
        )
    );
}

#[test]
fn test_execute_if_score() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = r#"execute if score max test_global >= #min test_global run function test:func"#;

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
            },
            None
        )
    );
}

#[test]
fn test_execute_if_score_matches() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = r#"execute if score * test_global matches 0.. run function test:func"#;

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
            },
            None
        )
    );
}

#[test]
fn test_execute_store_block() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = r#"execute store success block 1 2 3 Items[0].Count byte 10 run function test:func"#;

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
            },
            None
        )
    );
}

#[test]
fn test_execute_store_bossbar() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = r#"execute store result bossbar test value run function test:func"#;

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
            },
            None
        )
    );
}

#[test]
fn test_execute_store_entity() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = r#"execute store success entity @e[type=minecraft:chest_minecart,sort=nearest,limit=1] Items[{id:"minecraft:apple"}].Count byte 10 run function test:func"#;

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![29],
            },
            None
        )
    );
}

#[test]
fn test_execute_store_score() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = r#"execute store success score @s test_global run function test:func"#;

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![28],
            },
            None
        )
    );
}

#[test]
fn test_execute_store_storage() {
    // given:
    let parser = CommandParser::default().unwrap();
    let line = r#"execute store result storage :test my_result long -.5 run function test:func"#;

    // when:
    let actual = parse_line_internal(&parser, line);

    // then:
    assert_eq!(
        actual,
        (
            Line::FunctionCall {
                name: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                anchor: None,
                execute_as: false,
                selectors: vec![],
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
                function: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                operation: ScheduleOperation::REPLACE {
                    time: MinecraftTime {
                        time: 1f32,
                        unit: MinecraftTimeUnit::Tick
                    }
                },
                selectors: vec![],
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
                function: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                operation: ScheduleOperation::APPEND {
                    time: MinecraftTime {
                        time: 1f32,
                        unit: MinecraftTimeUnit::Tick
                    }
                },
                selectors: vec![],
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
                function: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                operation: ScheduleOperation::CLEAR,
                selectors: vec![],
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
                function: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                operation: ScheduleOperation::REPLACE {
                    time: MinecraftTime {
                        time: 1f32,
                        unit: MinecraftTimeUnit::Tick
                    }
                },
                selectors: vec![],
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
                function: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
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
                function: ResourceLocationRef::try_from("test:func")
                    .unwrap()
                    .to_owned(),
                operation: ScheduleOperation::CLEAR,
                selectors: vec![11],
            },
            None
        )
    );
}
