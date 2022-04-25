use super::*;
use std::{convert::TryFrom, iter::FromIterator};

#[test]
fn test_type_inverted() {
    // given:
    let line = r#"@e[ type = ! player ] bla"#;

    // when:
    let actual = MinecraftSelector::parse(line).unwrap();

    // then:
    let mut expected = MinecraftSelector::new(MinecraftSelectorType::E);
    expected.entity_type = Some(EntityType {
        inverted: true,
        tag: false,
        resource_location: ResourceLocationRef::try_from("minecraft:player").unwrap(),
    });
    assert_eq!(actual, (expected, 21));
}

#[test]
fn test_type_inverted_tag() {
    // given:
    let line = r#"@e[ type = ! # player ] bla"#;

    // when:
    let actual = MinecraftSelector::parse(line).unwrap();

    // then:
    let mut expected = MinecraftSelector::new(MinecraftSelectorType::E);
    expected.entity_type = Some(EntityType {
        inverted: true,
        tag: true,
        resource_location: ResourceLocationRef::try_from("minecraft:player").unwrap(),
    });
    assert_eq!(actual, (expected, 23));
}

#[test]
fn test_scores_no_comma() {
    // given:
    let line = r#"@e[ scores = { a = -4.. b = 5 } ] bla"#;

    // when:
    let actual = MinecraftSelector::parse(line).unwrap();

    // then:
    let mut expected = MinecraftSelector::new(MinecraftSelectorType::E);
    expected.scores = BTreeMap::from_iter([
        (
            "a",
            MinecraftRange {
                min: Some(-4),
                max: None,
            },
        ),
        (
            "b",
            MinecraftRange {
                min: Some(5),
                max: Some(5),
            },
        ),
    ]);
    assert_eq!(actual, (expected, 33));
}

#[test]
fn test_advancements_no_comma() {
    // given:
    let line = r#"@e[ advancements = { a = true b = false } ] bla"#;

    // when:
    let actual = MinecraftSelector::parse(line).unwrap();

    // then:
    let mut expected = MinecraftSelector::new(MinecraftSelectorType::E);
    expected.advancements = BTreeMap::from_iter([
        (
            ResourceLocationRef::try_from("minecraft:a").unwrap(),
            MinecraftAdvancementProgress::AdvancementProgress(true),
        ),
        (
            ResourceLocationRef::try_from("minecraft:b").unwrap(),
            MinecraftAdvancementProgress::AdvancementProgress(false),
        ),
    ]);
    assert_eq!(actual, (expected, 43));
}

#[test]
fn test_advancement_criteria_no_comma() {
    // given:
    let line = r#"@e[ advancements = { a = { b = true c = false } } ] bla"#;

    // when:
    let actual = MinecraftSelector::parse(line).unwrap();

    // then:
    let mut expected = MinecraftSelector::new(MinecraftSelectorType::E);
    expected.advancements = BTreeMap::from_iter([(
        ResourceLocationRef::try_from("minecraft:a").unwrap(),
        MinecraftAdvancementProgress::CriterionProgress(BTreeMap::from_iter([
            ("b", true),
            ("c", false),
        ])),
    )]);
    assert_eq!(actual, (expected, 51));
}

#[test]
fn test_unknown() {
    // given:
    let line = r#"@e[ unknown = ! abc .. 1234 + , limit = 4 ] bla"#;

    // when:
    let actual = MinecraftSelector::parse(line).unwrap();

    // then:
    let mut expected = MinecraftSelector::new(MinecraftSelectorType::E);
    expected.limit = Some(4);
    assert_eq!(actual, (expected, 43));
}
