use const_format::concatcp;
use log::warn;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display, u32, usize};

pub struct CommandParser {
    commands: HashMap<String, Command>,
}

impl CommandParser {
    pub fn default() -> Result<CommandParser, serde_json::Error> {
        let json = include_str!("commands.json");
        CommandParser::from_str(json)
    }

    pub fn from_str(json: &str) -> serde_json::Result<CommandParser> {
        let root_node: RootNode = serde_json::from_str(json)?;
        Ok(CommandParser {
            commands: root_node.children,
        })
    }

    pub fn parse<'l>(&'l self, mut string: &'l str) -> Result<Vec<ParsedNode<'l>>, String> {
        let mut vec = Vec::new();
        let mut commands = &self.commands;

        let mut index = 0;
        loop {
            let (command, parsed, suffix) = CommandParser::parse_prefix(string, index, commands)?;
            vec.push(parsed);

            if suffix == "" {
                if command.executable() {
                    return Ok(vec);
                } else {
                    // TODO error handling
                    return Err(
                        "Incomplete command, see below for error\n...hored eyes<--[HERE]"
                            .to_string(),
                    );
                }
            } else {
                index += string.len();
                string = suffix.strip_prefix(' ').ok_or(
                    "Expected whitespace to end one argument, but found trailing data at position 22: ...hored eyes#<--[HERE]".to_string()
                )?;
                index -= string.len();

                commands = command.children();
                if let Some(redirect) = command.redirect()? {
                    let command = self
                        .commands
                        .get(redirect)
                        .ok_or(format!("Failed to resolve redirect {}", redirect))?;
                    vec.push(ParsedNode::Redirect(redirect));
                    commands = command.children();
                } else if commands.is_empty() {
                    if !command.executable() {
                        // Special case for execute run which has no redirect to root for some reason
                        commands = &self.commands;
                    } else {
                        return Err(
                            "Incorrect argument for command at position 13: ...me set day<--[HERE]"
                                .to_string(),
                        );
                    }
                }
            }
        }
    }

    fn parse_prefix<'c, 's>(
        string: &'s str,
        index: usize,
        commands: &'c HashMap<String, Command>,
    ) -> Result<(&'c Command, ParsedNode<'s>, &'s str), String> {
        for (name, command) in commands {
            if let Some((parsed, suffix)) = command.parse(name, string, index) {
                return Ok((command, parsed, suffix));
            }
        }
        // TODO error handling
        Err("Unknown command, see below for error\nabcd<--[HERE]".to_string())
    }
}

pub enum ParsedNode<'l> {
    Redirect(&'l str),
    Literal {
        literal: &'l str,
        index: usize,
    },
    Argument {
        argument: Argument<'l>,
        index: usize,
    },
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type", rename = "root")]
struct RootNode {
    children: HashMap<String, Command>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Literal {
        #[serde(flatten)]
        node: Node,
    },
    Argument {
        #[serde(flatten)]
        node: Node,
        #[serde(flatten)]
        parser: ArgumentParser,
    },
}

impl Command {
    fn parse<'l>(
        &self,
        name: &str,
        string: &'l str,
        index: usize,
    ) -> Option<(ParsedNode<'l>, &'l str)> {
        match self {
            Command::Literal { .. } => {
                let end = string.find(' ').unwrap_or(string.len());
                let (literal, suffix) = string.split_at(end);
                if literal == name {
                    Some((ParsedNode::Literal { literal, index }, suffix))
                } else {
                    None
                }
            }
            Command::Argument { parser, .. } => {
                let (argument, suffix) = parser
                    .parse(string)
                    .map_err(|e| warn!("Failed to parse argument {} due to: {}", name, e))
                    .ok()?;
                Some((ParsedNode::Argument { argument, index }, suffix))
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Node {
    #[serde(default)]
    pub children: HashMap<String, Command>,
    #[serde(default)]
    pub executable: bool,
    #[serde(default)]
    pub redirect: Vec<String>,
}

impl Command {
    pub fn children(&self) -> &HashMap<String, Command> {
        match self {
            Command::Literal { node, .. } => &node.children,
            Command::Argument { node, .. } => &node.children,
        }
    }

    pub fn executable(&self) -> bool {
        match self {
            Command::Literal { node, .. } => node.executable,
            Command::Argument { node, .. } => node.executable,
        }
    }

    pub fn redirect(&self) -> Result<Option<&String>, String> {
        let redirect = match self {
            Command::Literal { node, .. } => &node.redirect,
            Command::Argument { node, .. } => &node.redirect,
        };
        if redirect.len() > 1 {
            Err(format!("Multi redirect is not supported: {:?}", redirect))
        } else {
            Ok(redirect.first())
        }
    }
}

type MinecraftEntity = ();
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MinecraftEntityAnchor {
    EYES,
    FEET,
}
type MinecraftFunction<'l> = NamespacedNameRef<&'l str>;
type MinecraftSwizzle = ();
#[derive(Clone, Debug, PartialEq)]
pub struct MinecraftTime {
    pub time: f32,
    pub unit: MinecraftTimeUnit,
}

impl MinecraftTime {
    pub fn as_ticks(&self) -> u32 {
        let ticks = self.time * self.unit.factor() as f32;
        ticks.round() as u32
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MinecraftTimeUnit {
    Tick,
    Second,
    Day,
}

impl MinecraftTimeUnit {
    pub fn factor(&self) -> u32 {
        match self {
            MinecraftTimeUnit::Tick => 1,
            MinecraftTimeUnit::Second => 20,
            MinecraftTimeUnit::Day => 24000,
        }
    }
}

pub enum Argument<'l> {
    BrigadierString(String),
    MinecraftEntity(MinecraftEntity),
    MinecraftEntityAnchor(MinecraftEntityAnchor),
    MinecraftFunction(MinecraftFunction<'l>),
    MinecraftSwizzle(MinecraftSwizzle),
    MinecraftTime(MinecraftTime),
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "parser", content = "properties")]
pub enum ArgumentParser {
    #[serde(rename = "brigadier:bool")]
    BrigadierBool,
    #[serde(rename = "brigadier:double")]
    BrigadierDouble,
    #[serde(rename = "brigadier:float")]
    BrigadierFloat(Option<BrigadierFloatProperties>),
    #[serde(rename = "brigadier:integer")]
    BrigadierInteger(Option<BrigadierIntegerProperties>),
    #[serde(rename = "brigadier:string")]
    BrigadierString {
        #[serde(rename = "type")]
        type_: BrigadierStringType,
    },
    #[serde(rename = "minecraft:angle")]
    MinecraftAngle,
    #[serde(rename = "minecraft:block_pos")]
    MinecraftBlockPos,
    #[serde(rename = "minecraft:block_predicate")]
    MinecraftBlockPredicate,
    #[serde(rename = "minecraft:block_state")]
    MinecraftBlockState,
    #[serde(rename = "minecraft:color")]
    MinecraftColor,
    #[serde(rename = "minecraft:column_pos")]
    MinecraftColumnPos,
    #[serde(rename = "minecraft:component")]
    MinecraftComponent,
    #[serde(rename = "minecraft:dimension")]
    MinecraftDimension,
    #[serde(rename = "minecraft:entity")]
    MinecraftEntity {
        #[serde(rename = "type")]
        type_: MinecraftEntityType,
        amount: MinecraftAmount,
    },
    #[serde(rename = "minecraft:entity_anchor")]
    MinecraftEntityAnchor,
    #[serde(rename = "minecraft:entity_summon")]
    MinecraftEntitySummon,
    #[serde(rename = "minecraft:function")]
    MinecraftFunction,
    #[serde(rename = "minecraft:game_profile")]
    MinecraftGameProfile,
    #[serde(rename = "minecraft:int_range")]
    MinecraftIntRange,
    #[serde(rename = "minecraft:item_enchantment")]
    MinecraftItemEnchantment,
    #[serde(rename = "minecraft:item_predicate")]
    MinecraftItemPredicate,
    #[serde(rename = "minecraft:item_slot")]
    MinecraftItemSlot,
    #[serde(rename = "minecraft:item_stack")]
    MinecraftItemStack,
    #[serde(rename = "minecraft:message")]
    MinecraftMessage,
    #[serde(rename = "minecraft:mob_effect")]
    MinecraftMobEffect,
    #[serde(rename = "minecraft:nbt_compound_tag")]
    MinecraftNbtCompoundTag,
    #[serde(rename = "minecraft:nbt_path")]
    MinecraftNbtPath,
    #[serde(rename = "minecraft:nbt_tag")]
    MinecraftNbtTag,
    #[serde(rename = "minecraft:objective")]
    MinecraftObjective,
    #[serde(rename = "minecraft:objective_criteria")]
    MinecraftObjectiveCriteria,
    #[serde(rename = "minecraft:operation")]
    MinecraftOperation,
    #[serde(rename = "minecraft:particle")]
    MinecraftParticle,
    #[serde(rename = "minecraft:resource_location")]
    MinecraftResourceLocation,
    #[serde(rename = "minecraft:rotation")]
    MinecraftRotation,
    #[serde(rename = "minecraft:score_holder")]
    MinecraftScoreHolder { amount: MinecraftAmount },
    #[serde(rename = "minecraft:scoreboard_slot")]
    MinecraftScoreboardSlot,
    #[serde(rename = "minecraft:swizzle")]
    MinecraftSwizzle,
    #[serde(rename = "minecraft:team")]
    MinecraftTeam,
    #[serde(rename = "minecraft:time")]
    MinecraftTime,
    #[serde(rename = "minecraft:uuid")]
    MinecraftUuid,
    #[serde(rename = "minecraft:vec2")]
    MinecraftVec2,
    #[serde(rename = "minecraft:vec3")]
    MinecraftVec3,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BrigadierFloatProperties {
    pub min: Option<f32>,
    pub max: Option<f32>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BrigadierIntegerProperties {
    pub min: Option<i32>,
    pub max: Option<i32>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum BrigadierStringType {
    Greedy,
    Phrase,
    Word,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum MinecraftEntityType {
    Players,
    Entities,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum MinecraftAmount {
    Single,
    Multiple,
}

impl ArgumentParser {
    pub fn parse<'l>(&self, string: &'l str) -> Result<(Argument<'l>, &'l str), String> {
        match self {
            ArgumentParser::BrigadierString { type_ } => {
                let (string, suffix) = ArgumentParser::parse_brigadier_string(string, type_)?;
                Ok((Argument::BrigadierString(string), suffix))
            }
            ArgumentParser::MinecraftEntity { .. } => {
                let (entity, suffix) = ArgumentParser::parse_minecraft_entity(string)?;
                Ok((Argument::MinecraftEntity(entity), suffix))
            }
            ArgumentParser::MinecraftEntityAnchor => {
                let (entity_anchor, suffix) =
                    ArgumentParser::parse_minecraft_entity_anchor(string)?;
                Ok((Argument::MinecraftEntityAnchor(entity_anchor), suffix))
            }
            ArgumentParser::MinecraftFunction => {
                let (function, suffix) = ArgumentParser::parse_minecraft_function(string)?;
                Ok((Argument::MinecraftFunction(function), suffix))
            }
            ArgumentParser::MinecraftSwizzle => {
                let (swizzle, suffix) = ArgumentParser::parse_minecraft_swizzle(string)?;
                Ok((Argument::MinecraftSwizzle(swizzle), suffix))
            }
            ArgumentParser::MinecraftTime => {
                let (time, suffix) = ArgumentParser::parse_minecraft_time(string)?;
                Ok((Argument::MinecraftTime(time), suffix))
            }
            _ => Err("Unknown argument".to_string()),
        }
    }

    fn parse_brigadier_string<'l>(
        string: &'l str,
        type_: &BrigadierStringType,
    ) -> Result<(String, &'l str), String> {
        match type_ {
            BrigadierStringType::Greedy => Ok((string.to_string(), "")),
            BrigadierStringType::Phrase => {
                Err("Unsupported type 'phrase' for argument parser brigadier:string".to_string())
            }
            BrigadierStringType::Word => {
                Err("Unsupported type 'word' for argument parser brigadier:string".to_string())
            }
        }
    }

    // TODO support ] in strings and NBT
    // TODO support for player name and UUID
    // TODO add support for limits on amount and type
    fn parse_minecraft_entity(string: &str) -> Result<(MinecraftEntity, &str), String> {
        let mut suffix = string
            .strip_prefix('@')
            .ok_or(format!("Invalid entity {}", string))?;

        if suffix.is_empty() {
            // TODO error handling
            return Err("Missing selector type".to_string());
        }

        suffix = suffix
            .strip_prefix(&['a', 'e', 'r', 's'][..])
            .ok_or(format!("Unknown selector type '{}'", &string[..2]))?;

        suffix = if let Some(suffix) = suffix.strip_prefix('[') {
            let end = suffix.find(']').ok_or(format!("Expected end of options"))?;
            &suffix[1 + end..]
        } else {
            &suffix
        };
        Ok(((), suffix))
    }

    fn parse_minecraft_entity_anchor(
        string: &str,
    ) -> Result<(MinecraftEntityAnchor, &str), String> {
        let eyes = "eyes";
        let feet = "feet";
        if string.starts_with(eyes) {
            Ok((MinecraftEntityAnchor::EYES, &string[eyes.len()..]))
        } else if string.starts_with(feet) {
            Ok((MinecraftEntityAnchor::FEET, &string[feet.len()..]))
        } else {
            Err(format!("Invalid entity anchor {}", string))
        }
    }

    fn parse_minecraft_function(string: &str) -> Result<(MinecraftFunction, &str), String> {
        let namespace_end = string
            .find(|c| !NAMESPACE_CHARS.contains(c))
            .ok_or(format!("Invalid ID: '{}'", string))?;
        let (_namespace, rest) = string.split_at(namespace_end);
        let rest = rest
            .strip_prefix(':')
            .ok_or(format!("Invalid ID: '{}'", string))?;
        let name_end = rest.find(|c| !NAME_CHARS.contains(c)).unwrap_or(rest.len());
        let len = namespace_end + 1 + name_end;
        let (string, rest) = string.split_at(len);
        let name = NamespacedNameRef {
            string,
            namespace_len: namespace_end,
        };
        Ok((name, rest))
    }

    fn parse_minecraft_swizzle(string: &str) -> Result<(MinecraftSwizzle, &str), String> {
        let end = string
            .find(' ')
            .ok_or("Failed to parse swizzle".to_string())?;
        let swizzle = ();
        Ok((swizzle, &string[end..]))
    }

    fn parse_minecraft_time(string: &str) -> Result<(MinecraftTime, &str), String> {
        let float_len = string
            .find(|c| c < '0' || c > '9' && c != '.' && c != '-')
            .unwrap_or(string.len());
        let float_sting = &string[..float_len];
        let time = float_sting
            .parse()
            .map_err(|_| format!("Expected float but got '{}'", &float_sting))?;
        let (unit, suffix) = match string[float_len..].chars().next() {
            Some(unit) if unit != ' ' => {
                let unit = match unit {
                    't' => MinecraftTimeUnit::Tick,
                    's' => MinecraftTimeUnit::Second,
                    'd' => MinecraftTimeUnit::Day,
                    unit => return Err(format!("Unknown unit '{}'", unit)),
                };
                (unit, &string[float_len + 1..])
            }
            _ => (MinecraftTimeUnit::Tick, &string[float_len..]),
        };

        Ok((MinecraftTime { time, unit }, suffix))
    }
}

const NAMESPACE_CHARS: &str = "0123456789abcdefghijklmnopqrstuvwxyz_-.";
const NAME_CHARS: &str = concatcp!(NAMESPACE_CHARS, "/");

pub type NamespacedName = NamespacedNameRef<String>;

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct NamespacedNameRef<S: AsRef<str>> {
    string: S,
    namespace_len: usize,
}

impl<S: AsRef<str>> NamespacedNameRef<S> {
    pub fn new(namespace: &str, name: &str) -> NamespacedName {
        NamespacedNameRef {
            string: format!("{}:{}", namespace, name),
            namespace_len: namespace.len(),
        }
    }

    pub fn from(string: S) -> Option<NamespacedNameRef<S>> {
        let namespace_len = string.as_ref().find(':')?;
        Some(NamespacedNameRef {
            string,
            namespace_len,
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json() {
        // when:
        let actual = &CommandParser::default().unwrap().commands;

        // then:
        assert!(
            actual.contains_key("execute"),
            "Expected actual to contain key 'execute': {:#?}",
            actual
        );
    }

    #[test]
    fn test_serialize() {
        // when:
        let root = RootNode {
            children: HashMap::new(),
        };

        let actual = serde_json::to_string(&root).unwrap();

        // then:
        assert_eq!(actual, r#"{"type":"root","children":{}}"#);
    }
}
