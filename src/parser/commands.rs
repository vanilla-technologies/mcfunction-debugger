use const_format::concatcp;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{Display, Write},
    ops::Not,
    u32, usize,
};

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

    pub fn parse<'l>(
        &'l self,
        command: &'l str,
    ) -> Result<Vec<ParsedNode<'l>>, CommandParserError<'l>> {
        let mut vec = Vec::new();
        let mut commands = &self.commands;

        let mut index = 0;
        loop {
            let (command_spec, parsed, parsed_len) =
                CommandParser::parse_from(command, index, commands)?;
            vec.push(parsed);
            index += parsed_len;

            if index >= command.len() {
                if command_spec.executable() {
                    return Ok(vec);
                } else {
                    let message = "Incomplete command, see below for error\n...hored eyes<--[HERE]"
                        .to_string();
                    return Err(CommandParserError {
                        message,
                        command,
                        index,
                    });
                }
            } else {
                let message = "Expected whitespace to end one argument, but found trailing data at position 22: ...hored eyes#<--[HERE]".to_string();
                const SPACE: char = ' ';
                command[index..]
                    .starts_with(SPACE)
                    .then(|| ())
                    .ok_or(CommandParserError {
                        message,
                        command,
                        index,
                    })?;
                index += SPACE.len_utf8();

                commands = command_spec.children();
                if let Some(redirect) =
                    command_spec
                        .redirect()
                        .map_err(|message| CommandParserError {
                            message,
                            command,
                            index,
                        })?
                {
                    let command = self.commands.get(redirect).ok_or(CommandParserError {
                        message: format!("Failed to resolve redirect {}", redirect),
                        command,
                        index,
                    })?;
                    vec.push(ParsedNode::Redirect(redirect));
                    commands = command.children();
                } else if commands.is_empty() {
                    if !command_spec.executable() {
                        // Special case for execute run which has no redirect to root for some reason
                        commands = &self.commands;
                    } else {
                        let message =
                            "Incorrect argument for command at position 13: ...me set day<--[HERE]"
                                .to_string();
                        return Err(CommandParserError {
                            message,
                            command,
                            index,
                        });
                    }
                }
            }
        }
    }

    fn parse_from<'c, 's>(
        command: &'s str,
        index: usize,
        commands: &'c HashMap<String, Command>,
    ) -> Result<(&'c Command, ParsedNode<'s>, usize), CommandParserError<'s>> {
        // Try to parse as literal
        let string = &command[index..];
        let len = string.find(' ').unwrap_or(string.len());
        let literal = &string[..len];
        let command_spec = commands
            .iter()
            .find(|(name, command)| {
                name.as_str() == literal && matches!(command, Command::Literal { .. })
            })
            .map(|(_name, command)| command);
        if let Some(command) = command_spec {
            Ok((command, ParsedNode::Literal { literal, index }, len))
        } else {
            // try to parse as argument
            let mut parsed_arguments = commands
                .iter()
                .filter_map(|(_name, command)| match command {
                    Command::Literal { .. } => None,
                    Command::Argument { parser, .. } => Some((command, parser)),
                })
                .map(|(command, parser)| (command, parser.parse(string)))
                .collect::<Vec<_>>();
            // Prefer longest successful parsed
            parsed_arguments.sort_by_key(|(_command, r)| match r {
                Ok((_argument, len)) => -(*len as isize),
                Err(_) => 1,
            });
            let (command_spec, result) =
                parsed_arguments
                    .into_iter()
                    .next()
                    .ok_or(CommandParserError {
                        message: "Unknown command, see below for error\nabcd<--[HERE]".to_string(),
                        command,
                        index,
                    })?;
            let (argument, suffix) = result.map_err(|message| CommandParserError {
                message,
                command,
                index,
            })?;
            let parsed = ParsedNode::Argument { argument, index };
            Ok((command_spec, parsed, suffix))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandParserError<'l> {
    message: String,
    command: &'l str,
    index: usize,
}

impl Display for CommandParserError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:\n{}\n", self.message, self.command)?;
        for _ in 0..self.index {
            f.write_char(' ')?;
        }
        f.write_char('^')
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinecraftEntityAnchor {
    EYES,
    FEET,
}

type MinecraftFunction<'l> = NamespacedNameRef<&'l str>;

type MinecraftObjective<'l> = &'l str;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinecraftOperation {
    Assignment,     // =
    Addition,       // +=
    Subtraction,    // -=
    Multiplication, // *=
    Division,       // /=
    Modulus,        // %=
    Swapping,       // ><
    Minimum,        // <
    Maximum,        // >
}

type MinecraftRotation = ();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinecraftScoreHolder<'l> {
    Selector(MinecraftSelector),
    Wildcard,
    String(&'l str),
}

type MinecraftSelector = ();

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

type MinecraftVec3 = ();

pub enum Argument<'l> {
    BrigadierString(&'l str),
    MinecraftEntity(MinecraftEntity),
    MinecraftEntityAnchor(MinecraftEntityAnchor),
    MinecraftFunction(MinecraftFunction<'l>),
    MinecraftObjective(MinecraftObjective<'l>),
    MinecraftOperation(MinecraftOperation),
    MinecraftRotation(MinecraftRotation),
    MinecraftScoreHolder(MinecraftScoreHolder<'l>),
    MinecraftSwizzle(MinecraftSwizzle),
    MinecraftTime(MinecraftTime),
    MinecraftVec3(MinecraftVec3),
    Unknown(&'l str),
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
    #[serde(other)]
    Unknown,
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

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
    fn name(&self) -> Option<String> {
        let a = serde_json::to_value(self).ok()?;
        a.as_object()?.get("parser")?.as_str().map(String::from)
    }

    pub fn parse<'l>(&self, string: &'l str) -> Result<(Argument<'l>, usize), String> {
        match self {
            ArgumentParser::BrigadierString { type_ } => {
                let (string, len) = ArgumentParser::parse_brigadier_string(string, *type_)?;
                Ok((Argument::BrigadierString(string), len))
            }
            ArgumentParser::MinecraftEntity { .. } => {
                let (entity, len) = ArgumentParser::parse_minecraft_entity(string)?;
                Ok((Argument::MinecraftEntity(entity), len))
            }
            ArgumentParser::MinecraftEntityAnchor => {
                let (entity_anchor, len) = ArgumentParser::parse_minecraft_entity_anchor(string)?;
                Ok((Argument::MinecraftEntityAnchor(entity_anchor), len))
            }
            ArgumentParser::MinecraftFunction => {
                let (function, len) = ArgumentParser::parse_minecraft_function(string)?;
                Ok((Argument::MinecraftFunction(function), len))
            }
            ArgumentParser::MinecraftObjective => {
                let (objective, len) = ArgumentParser::parse_minecraft_objective(string)?;
                Ok((Argument::MinecraftObjective(objective), len))
            }
            ArgumentParser::MinecraftOperation => {
                let (operation, len) = ArgumentParser::parse_minecraft_operation(string)?;
                Ok((Argument::MinecraftOperation(operation), len))
            }
            ArgumentParser::MinecraftRotation => {
                let (rotation, len) = ArgumentParser::parse_minecraft_rotation(string)?;
                Ok((Argument::MinecraftRotation(rotation), len))
            }
            ArgumentParser::MinecraftScoreHolder { .. } => {
                let (score_holder, len) = ArgumentParser::parse_minecraft_score_holder(string)?;
                Ok((Argument::MinecraftScoreHolder(score_holder), len))
            }
            ArgumentParser::MinecraftSwizzle => {
                let (swizzle, len) = ArgumentParser::parse_minecraft_swizzle(string)?;
                Ok((Argument::MinecraftSwizzle(swizzle), len))
            }
            ArgumentParser::MinecraftTime => {
                let (time, len) = ArgumentParser::parse_minecraft_time(string)?;
                Ok((Argument::MinecraftTime(time), len))
            }
            ArgumentParser::MinecraftVec3 => {
                let (vec3, len) = ArgumentParser::parse_minecraft_vec3(string)?;
                Ok((Argument::MinecraftVec3(vec3), len))
            }
            ArgumentParser::Unknown => {
                let (unknown, len) = ArgumentParser::parse_unknown(string)?;
                Ok((Argument::Unknown(unknown), len))
            }
            _ => Err(format!(
                "Unsupported argument type: {}",
                self.name().unwrap_or_default()
            )),
        }
    }

    fn parse_brigadier_double(string: &str) -> Result<(f64, usize), String> {
        let len = string
            .find(|c| (c < '0' || c > '9') && c != '.' && c != '-')
            .unwrap_or(string.len());
        if len == 0 {
            Ok((0.0, len))
        } else {
            let f = &string[..len];
            let f = f.parse::<f64>().map_err(|e| e.to_string())?;
            Ok((f, len))
        }
    }

    fn parse_brigadier_string(
        string: &str,
        type_: BrigadierStringType,
    ) -> Result<(&str, usize), String> {
        match type_ {
            BrigadierStringType::Greedy => Ok((string, string.len())),
            BrigadierStringType::Phrase => {
                Err("Unsupported type 'phrase' for argument parser brigadier:string".to_string())
            }
            BrigadierStringType::Word => parse_unquoted_string(string),
        }
    }

    // TODO support for player name and UUID
    fn parse_minecraft_entity(string: &str) -> Result<(MinecraftEntity, usize), String> {
        parse_minecraft_selector(string)
    }

    fn parse_minecraft_entity_anchor(
        string: &str,
    ) -> Result<(MinecraftEntityAnchor, usize), String> {
        let eyes = "eyes";
        let feet = "feet";
        if string.starts_with(eyes) {
            Ok((MinecraftEntityAnchor::EYES, eyes.len()))
        } else if string.starts_with(feet) {
            Ok((MinecraftEntityAnchor::FEET, feet.len()))
        } else {
            Err("Invalid entity anchor".to_string())
        }
    }

    fn parse_minecraft_function(string: &str) -> Result<(MinecraftFunction, usize), String> {
        let namespace_end = string
            .find(|c| !NAMESPACE_CHARS.contains(c))
            .ok_or(format!("Invalid ID: '{}'", string))?;
        let (_namespace, rest) = string.split_at(namespace_end);
        let rest = rest
            .strip_prefix(':')
            .ok_or(format!("Invalid ID: '{}'", string))?;
        let name_end = rest.find(|c| !NAME_CHARS.contains(c)).unwrap_or(rest.len());
        let len = namespace_end + 1 + name_end;
        let name = NamespacedNameRef {
            string: &string[..len],
            namespace_len: namespace_end,
        };
        Ok((name, len))
    }

    fn parse_minecraft_objective(string: &str) -> Result<(MinecraftObjective, usize), String> {
        parse_unquoted_string(string)
    }

    fn parse_minecraft_operation(string: &str) -> Result<(MinecraftOperation, usize), String> {
        let len = string.find(' ').unwrap_or(string.len());
        match &string[..len] {
            "=" => Ok((MinecraftOperation::Assignment, len)),
            "+=" => Ok((MinecraftOperation::Addition, len)),
            "-=" => Ok((MinecraftOperation::Subtraction, len)),
            "*=" => Ok((MinecraftOperation::Multiplication, len)),
            "/=" => Ok((MinecraftOperation::Division, len)),
            "%=" => Ok((MinecraftOperation::Modulus, len)),
            ">< " => Ok((MinecraftOperation::Swapping, len)),
            "<" => Ok((MinecraftOperation::Minimum, len)),
            ">" => Ok((MinecraftOperation::Maximum, len)),
            _ => Err("Invalid operation".to_string()),
        }
    }

    fn parse_minecraft_rotation(string: &str) -> Result<(MinecraftRotation, usize), String> {
        const INCOMPLETE: &str = "Incomplete (expected 3 coordinates)";
        let suffix = string.strip_prefix('~').unwrap_or(string);
        let (_x, len) = ArgumentParser::parse_brigadier_double(suffix)?;
        let suffix = &suffix[len..];
        let suffix = suffix.strip_prefix(' ').ok_or(INCOMPLETE.to_string())?;
        check_non_local(suffix)?;
        let suffix = suffix.strip_prefix('~').unwrap_or(suffix);
        let (_y, len) = ArgumentParser::parse_brigadier_double(suffix)?;

        Ok(((), string.len() - &suffix[len..].len()))
    }

    fn parse_minecraft_score_holder(string: &str) -> Result<(MinecraftScoreHolder, usize), String> {
        if string.starts_with('@') {
            let (selector, len) = parse_minecraft_selector(string)?;
            Ok((MinecraftScoreHolder::Selector(selector), len))
        } else {
            let len = string.find(' ').unwrap_or(string.len());
            let parsed = &string[..len];
            let parsed = if parsed == "*" {
                MinecraftScoreHolder::Wildcard
            } else {
                MinecraftScoreHolder::String(parsed)
            };
            Ok((parsed, len))
        }
    }

    fn parse_minecraft_swizzle(string: &str) -> Result<(MinecraftSwizzle, usize), String> {
        let len = string
            .find(' ')
            .ok_or("Failed to parse swizzle".to_string())?;
        let swizzle = ();
        Ok((swizzle, len))
    }

    fn parse_minecraft_time(string: &str) -> Result<(MinecraftTime, usize), String> {
        let float_len = string
            .find(|c| c < '0' || c > '9' && c != '.' && c != '-')
            .unwrap_or(string.len());
        let float_sting = &string[..float_len];
        let time = float_sting
            .parse()
            .map_err(|_| format!("Expected float but got '{}'", &float_sting))?;
        let (unit, len) = match string[float_len..].chars().next() {
            Some(unit) if unit != ' ' => {
                let unit = match unit {
                    't' => MinecraftTimeUnit::Tick,
                    's' => MinecraftTimeUnit::Second,
                    'd' => MinecraftTimeUnit::Day,
                    unit => return Err(format!("Unknown unit '{}'", unit)),
                };
                (unit, float_len + 1)
            }
            _ => (MinecraftTimeUnit::Tick, float_len),
        };

        Ok((MinecraftTime { time, unit }, len))
    }

    fn parse_minecraft_vec3(string: &str) -> Result<(MinecraftVec3, usize), String> {
        const INCOMPLETE: &str = "Incomplete (expected 3 coordinates)";
        let suffix = if let Some(suffix) = string.strip_prefix('^') {
            let (_x, len) = ArgumentParser::parse_brigadier_double(suffix)?;
            let suffix = &suffix[len..];
            let suffix = suffix.strip_prefix(' ').ok_or(INCOMPLETE.to_string())?;
            let suffix = suffix.strip_prefix('^').ok_or(CANNOT_MIX.to_string())?;
            let (_y, len) = ArgumentParser::parse_brigadier_double(suffix)?;
            let suffix = &suffix[len..];
            let suffix = suffix.strip_prefix(' ').ok_or(INCOMPLETE.to_string())?;
            let suffix = suffix.strip_prefix('^').ok_or(CANNOT_MIX.to_string())?;
            let (_z, len) = ArgumentParser::parse_brigadier_double(suffix)?;
            let suffix = &suffix[len..];
            suffix
        } else {
            let suffix = string.strip_prefix('~').unwrap_or(string);
            let (_x, len) = ArgumentParser::parse_brigadier_double(suffix)?;
            let suffix = &suffix[len..];
            let suffix = suffix.strip_prefix(' ').ok_or(INCOMPLETE.to_string())?;
            check_non_local(suffix)?;
            let suffix = suffix.strip_prefix('~').unwrap_or(suffix);
            let (_y, len) = ArgumentParser::parse_brigadier_double(suffix)?;
            let suffix = &suffix[len..];
            let suffix = suffix.strip_prefix(' ').ok_or(INCOMPLETE.to_string())?;
            check_non_local(suffix)?;
            let suffix = suffix.strip_prefix('~').unwrap_or(suffix);
            let (_z, len) = ArgumentParser::parse_brigadier_double(suffix)?;
            let suffix = &suffix[len..];
            suffix
        };
        Ok(((), string.len() - suffix.len()))
    }

    fn parse_unknown(string: &str) -> Result<(&str, usize), String> {
        // Best effort
        let len = string.find(' ').unwrap_or(string.len());
        Ok((&string[..len], len))
    }
}

fn is_allowed_in_unquoted_string(c: char) -> bool {
    return c >= '0' && c <= '9'
        || c >= 'A' && c <= 'Z'
        || c >= 'a' && c <= 'z'
        || c == '_'
        || c == '-'
        || c == '.'
        || c == '+';
}

fn parse_unquoted_string(string: &str) -> Result<(&str, usize), String> {
    let len = string
        .find(|c| !is_allowed_in_unquoted_string(c))
        .unwrap_or(string.len());
    Ok((&string[..len], len))
}

// TODO support ] in strings and NBT
fn parse_minecraft_selector(string: &str) -> Result<(MinecraftSelector, usize), String> {
    let mut suffix = string
        .strip_prefix('@')
        .ok_or(format!("Invalid entity {}", string))?;

    if suffix.is_empty() {
        return Err("Missing selector type".to_string());
    }

    suffix = suffix
        .strip_prefix(&['a', 'e', 'r', 's'][..])
        .ok_or(format!(
            "Unknown selector type '{}'",
            suffix.chars().next().unwrap()
        ))?;

    suffix = if let Some(suffix) = suffix.strip_prefix('[') {
        let end = suffix.find(']').ok_or(format!("Expected end of options"))?;
        &suffix[1 + end..]
    } else {
        &suffix
    };
    Ok(((), string.len() - suffix.len()))
}

const CANNOT_MIX: &str =
    "Cannot mix world & local coordinates (everyhing must either use ^ or not)";
fn check_non_local(string: &str) -> Result<(), String> {
    string
        .starts_with('^')
        .not()
        .then(|| ())
        .ok_or(CANNOT_MIX.to_string())
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
