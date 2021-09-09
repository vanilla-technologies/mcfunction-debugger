// mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any
// Minecraft mods.
//
// © Copyright (C) 2021 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
//
// This file is part of mcfunction-debugger.
//
// mcfunction-debugger is free software: you can redistribute it and/or modify it under the terms of
// the GNU General Public License as published by the Free Software Foundation, either version 3 of
// the License, or (at your option) any later version.
//
// mcfunction-debugger is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with mcfunction-debugger.
// If not, see <http://www.gnu.org/licenses/>.

pub mod brigadier;
pub mod coordinate;

use self::{
    brigadier::BrigadierStringType,
    coordinate::{MinecraftBlockPos, MinecraftRotation, MinecraftVec3},
};
use crate::parser::command::resource_location::ResourceLocationRef;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fmt::Display, u32, usize};

type MinecraftDimension<'l> = ResourceLocationRef<&'l str>;

type MinecraftEntity = ();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinecraftEntityAnchor {
    EYES,
    FEET,
}

type MinecraftFunction<'l> = ResourceLocationRef<&'l str>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinecraftIntRange {
    pub min: Option<i32>,
    pub max: Option<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinecraftMessage<'l> {
    pub message: &'l str,
    pub selectors: Vec<(MinecraftSelector, usize, usize)>,
}

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

type MinecraftResourceLocation<'l> = ResourceLocationRef<&'l str>;

#[derive(Clone, Debug, Eq, PartialEq)]
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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
pub enum Argument<'l> {
    BrigadierDouble(f64),
    BrigadierString(&'l str),
    MinecraftBlockPos(MinecraftBlockPos),
    MinecraftDimension(MinecraftDimension<'l>),
    MinecraftEntity(MinecraftEntity),
    MinecraftEntityAnchor(MinecraftEntityAnchor),
    MinecraftFunction(MinecraftFunction<'l>),
    MinecraftIntRange(MinecraftIntRange),
    MinecraftMessage(MinecraftMessage<'l>),
    MinecraftObjective(MinecraftObjective<'l>),
    MinecraftOperation(MinecraftOperation),
    MinecraftResourceLocation(MinecraftResourceLocation<'l>),
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
            ArgumentParser::BrigadierDouble => {
                let (argument, len) = brigadier::parse_double(string)?;
                Ok((Argument::BrigadierDouble(argument), len))
            }
            ArgumentParser::BrigadierString { type_ } => {
                let (argument, len) = brigadier::parse_string(string, *type_)?;
                Ok((Argument::BrigadierString(argument), len))
            }
            ArgumentParser::MinecraftBlockPos => {
                let (argument, len) = MinecraftBlockPos::parse(string)?;
                Ok((Argument::MinecraftBlockPos(argument), len))
            }
            ArgumentParser::MinecraftDimension => {
                let (argument, len) = parse_minecraft_dimension(string)?;
                Ok((Argument::MinecraftDimension(argument), len))
            }
            ArgumentParser::MinecraftEntity { .. } => {
                let (argument, len) = parse_minecraft_entity(string)?;
                Ok((Argument::MinecraftEntity(argument), len))
            }
            ArgumentParser::MinecraftEntityAnchor => {
                let (argument, len) = parse_minecraft_entity_anchor(string)?;
                Ok((Argument::MinecraftEntityAnchor(argument), len))
            }
            ArgumentParser::MinecraftFunction => {
                let (argument, len) = parse_minecraft_function(string)?;
                Ok((Argument::MinecraftFunction(argument), len))
            }
            ArgumentParser::MinecraftIntRange => {
                let (argument, len) = parse_minecraft_int_range(string)?;
                Ok((Argument::MinecraftIntRange(argument), len))
            }
            ArgumentParser::MinecraftMessage => {
                let (argument, len) = parse_minecraft_message(string)?;
                Ok((Argument::MinecraftMessage(argument), len))
            }
            ArgumentParser::MinecraftObjective => {
                let (argument, len) = parse_minecraft_objective(string)?;
                Ok((Argument::MinecraftObjective(argument), len))
            }
            ArgumentParser::MinecraftOperation => {
                let (argument, len) = parse_minecraft_operation(string)?;
                Ok((Argument::MinecraftOperation(argument), len))
            }
            ArgumentParser::MinecraftResourceLocation => {
                let (argument, len) = parse_minecraft_resource_location(string)?;
                Ok((Argument::MinecraftResourceLocation(argument), len))
            }
            ArgumentParser::MinecraftRotation => {
                let (argument, len) = MinecraftRotation::parse(string)?;
                Ok((Argument::MinecraftRotation(argument), len))
            }
            ArgumentParser::MinecraftScoreHolder { .. } => {
                let (argument, len) = parse_minecraft_score_holder(string)?;
                Ok((Argument::MinecraftScoreHolder(argument), len))
            }
            ArgumentParser::MinecraftSwizzle => {
                let (argument, len) = parse_minecraft_swizzle(string)?;
                Ok((Argument::MinecraftSwizzle(argument), len))
            }
            ArgumentParser::MinecraftTime => {
                let (argument, len) = parse_minecraft_time(string)?;
                Ok((Argument::MinecraftTime(argument), len))
            }
            ArgumentParser::MinecraftVec3 => {
                let (argument, len) = MinecraftVec3::parse(string)?;
                Ok((Argument::MinecraftVec3(argument), len))
            }
            ArgumentParser::Unknown => {
                let (argument, len) = parse_unknown(string)?;
                Ok((Argument::Unknown(argument), len))
            }
            _ => Err(format!(
                "Unsupported argument type: {}",
                self.name().unwrap_or_default()
            )),
        }
    }
}

fn parse_minecraft_dimension(string: &str) -> Result<(MinecraftDimension, usize), String> {
    parse_minecraft_resource_location(string)
}

// TODO support for player name and UUID
fn parse_minecraft_entity(string: &str) -> Result<(MinecraftEntity, usize), String> {
    parse_minecraft_selector(string).map_err(Into::into)
}

fn parse_minecraft_entity_anchor(string: &str) -> Result<(MinecraftEntityAnchor, usize), String> {
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
    parse_minecraft_resource_location(string)
}

fn parse_minecraft_int_range(string: &str) -> Result<(MinecraftIntRange, usize), String> {
    const EMPTY: &str = "Expected value or range of values";
    const SEPERATOR: &str = "..";

    fn is_allowed_number(c: char) -> bool {
        c >= '0' && c <= '9' || c == '-'
    }

    fn number_len(string: &str) -> usize {
        let mut index = 0;
        loop {
            let suffix = &string[index..];
            index += suffix
                .find(|c| !is_allowed_number(c))
                .unwrap_or(suffix.len());
            let suffix = &string[index..];
            if suffix.starts_with('.') && !suffix.starts_with(SEPERATOR) {
                index += '.'.len_utf8();
            } else {
                break index;
            }
        }
    }

    fn parse_i32(string: &str) -> Result<Option<i32>, String> {
        if string.is_empty() {
            Ok(None)
        } else {
            string
                .parse()
                .map(Some)
                .map_err(|_| format!("Invalid integer '{}'", string))
        }
    }

    let min_len = number_len(string);
    let (min, suffix) = string.split_at(min_len);
    let min = parse_i32(min)?;

    let (max, len) = if let Some(suffix) = suffix.strip_prefix(SEPERATOR) {
        let max_len = number_len(suffix);
        let max = parse_i32(&suffix[..max_len])?;
        (max, min_len + SEPERATOR.len() + max_len)
    } else {
        (min, min_len)
    };

    if min.is_none() && max.is_none() {
        Err(EMPTY.to_string())
    } else {
        Ok((MinecraftIntRange { min, max }, len))
    }
}

fn parse_minecraft_message(message: &str) -> Result<(MinecraftMessage, usize), String> {
    let mut index = 0;
    let mut selectors = Vec::new();
    while let Some(i) = &message[index..].find('@') {
        index += i;
        match parse_minecraft_selector(&message[index..]) {
            Ok((selector, len)) => {
                selectors.push((selector, index, index + len));
                index += len;
            }
            Err(
                MinecraftSelectorParserError::MissingSelectorType
                | MinecraftSelectorParserError::UnknownSelectorType(..),
            ) => {
                index += 1;
            }
            Err(e) => return Err(e.to_string()),
        }
    }

    Ok((MinecraftMessage { message, selectors }, message.len()))
}

fn parse_minecraft_objective(string: &str) -> Result<(MinecraftObjective, usize), String> {
    brigadier::parse_unquoted_string(string)
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

fn parse_minecraft_resource_location(
    string: &str,
) -> Result<(MinecraftResourceLocation, usize), String> {
    const INVALID_ID: &str = "Invalid ID";

    let len = string
        .find(|c| !is_allowed_in_resource_location(c))
        .unwrap_or(string.len());
    let resource_location = &string[..len];

    let resource_location =
        ResourceLocationRef::try_from(resource_location).map_err(|_| INVALID_ID.to_string())?;
    Ok((resource_location, len))
}

fn is_allowed_in_resource_location(c: char) -> bool {
    return c >= '0' && c <= '9'
        || c >= 'a' && c <= 'z'
        || c == '-'
        || c == '.'
        || c == '/'
        || c == ':'
        || c == '_';
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

fn parse_unknown(string: &str) -> Result<(&str, usize), String> {
    // Best effort
    let len = string.find(' ').unwrap_or(string.len());
    Ok((&string[..len], len))
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum MinecraftSelectorParserError {
    MissingSelectorType,
    UnknownSelectorType(char),
    Other(String),
}

impl Display for MinecraftSelectorParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSelectorType => f.write_str("Missing selector type"),
            Self::UnknownSelectorType(selector_type) => {
                write!(f, "Unknown selector type '{}'", selector_type)
            }
            Self::Other(message) => f.write_str(&message),
        }
    }
}

impl From<MinecraftSelectorParserError> for String {
    fn from(e: MinecraftSelectorParserError) -> Self {
        e.to_string()
    }
}

// TODO support ] in strings and NBT
fn parse_minecraft_selector(
    string: &str,
) -> Result<(MinecraftSelector, usize), MinecraftSelectorParserError> {
    type Error = MinecraftSelectorParserError;

    let mut suffix = string
        .strip_prefix('@')
        .ok_or(Error::Other(format!("Invalid entity {}", string)))?;

    if suffix.is_empty() {
        return Err(Error::MissingSelectorType);
    }

    const SELECTOR_TYPES: &[char] = &['a', 'e', 'p', 'r', 's'];

    suffix = suffix
        .strip_prefix(SELECTOR_TYPES)
        .ok_or(Error::UnknownSelectorType(suffix.chars().next().unwrap()))?;

    suffix = if let Some(suffix) = suffix.strip_prefix('[') {
        let end = suffix
            .find(']')
            .ok_or(Error::Other(format!("Expected end of options")))?;
        &suffix[1 + end..]
    } else {
        &suffix
    };
    Ok(((), string.len() - suffix.len()))
}
