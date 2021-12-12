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
pub mod minecraft;

use self::{
    brigadier::{parse_unquoted_string, BrigadierStringType},
    minecraft::{
        coordinate::{MinecraftBlockPos, MinecraftRotation, MinecraftVec3},
        entity::{MinecraftSelector, MinecraftSelectorParserError},
        nbt::MinecraftNbtPath,
        range::MinecraftRange,
    },
};
use crate::{
    parser::command::{
        argument::minecraft::{block::MinecraftBlockPredicate, entity::MinecraftEntity},
        resource_location::ResourceLocationRef,
    },
    utils::Map0,
};
use serde::{Deserialize, Serialize};
use std::{u32, usize};

type MinecraftDimension<'l> = ResourceLocationRef<&'l str>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinecraftEntityAnchor {
    EYES,
    FEET,
}

type MinecraftFunction<'l> = ResourceLocationRef<&'l str>;

#[derive(Clone, Debug, PartialEq)]
pub struct MinecraftMessage<'l> {
    pub message: &'l str,
    pub selectors: Vec<(MinecraftSelector<'l>, usize, usize)>,
}

type MinecraftObjective<'l> = &'l str;

type MinecraftObjectiveCriteria<'l> = &'l str;

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

#[derive(Clone, Debug, PartialEq)]
pub enum MinecraftScoreHolder<'l> {
    Selector(MinecraftSelector<'l>),
    Wildcard,
    String(&'l str),
}

type MinecraftScoreboardSlot<'l> = &'l str;

type MinecraftSwizzle = ();

type MinecraftTeam<'l> = &'l str;

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
    BrigadierInteger(i32),
    BrigadierString(&'l str),
    MinecraftBlockPos(MinecraftBlockPos),
    MinecraftBlockPredicate(MinecraftBlockPredicate<'l>),
    MinecraftDimension(MinecraftDimension<'l>),
    MinecraftEntity(MinecraftEntity<'l>),
    MinecraftEntityAnchor(MinecraftEntityAnchor),
    MinecraftFunction(MinecraftFunction<'l>),
    MinecraftIntRange(MinecraftRange<i32>),
    MinecraftMessage(MinecraftMessage<'l>),
    MinecraftNbtPath(MinecraftNbtPath<'l>),
    MinecraftObjective(MinecraftObjective<'l>),
    MinecraftObjectiveCriteria(MinecraftObjectiveCriteria<'l>),
    MinecraftOperation(MinecraftOperation),
    MinecraftResourceLocation(MinecraftResourceLocation<'l>),
    MinecraftRotation(MinecraftRotation),
    MinecraftScoreHolder(MinecraftScoreHolder<'l>),
    MinecraftScoreboardSlot(MinecraftScoreboardSlot<'l>),
    MinecraftSwizzle(MinecraftSwizzle),
    MinecraftTeam(MinecraftTeam<'l>),
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
            Self::BrigadierDouble => {
                brigadier::parse_double(string).map(|it| it.map0(Argument::BrigadierDouble))
            }
            Self::BrigadierInteger(..) => {
                brigadier::parse_integer(string).map(|it| it.map0(Argument::BrigadierInteger))
            }
            Self::BrigadierString { type_ } => {
                brigadier::parse_string(string, *type_).map(|it| it.map0(Argument::BrigadierString))
            }
            Self::MinecraftBlockPos => {
                MinecraftBlockPos::parse(string).map(|it| it.map0(Argument::MinecraftBlockPos))
            }
            Self::MinecraftBlockPredicate => MinecraftBlockPredicate::parse(string)
                .map(|it| it.map0(Argument::MinecraftBlockPredicate)),
            Self::MinecraftDimension => {
                MinecraftDimension::parse(string).map(|it| it.map0(Argument::MinecraftDimension))
            }
            Self::MinecraftEntity { .. } => {
                MinecraftEntity::parse(string).map(|it| it.map0(Argument::MinecraftEntity))
            }
            Self::MinecraftEntityAnchor => parse_minecraft_entity_anchor(string)
                .map(|it| it.map0(Argument::MinecraftEntityAnchor)),
            Self::MinecraftFunction => {
                MinecraftFunction::parse(string).map(|it| it.map0(Argument::MinecraftFunction))
            }
            Self::MinecraftIntRange => {
                MinecraftRange::parse(string).map(|it| it.map0(Argument::MinecraftIntRange))
            }
            Self::MinecraftMessage => {
                parse_minecraft_message(string).map(|it| it.map0(Argument::MinecraftMessage))
            }
            Self::MinecraftNbtPath => {
                MinecraftNbtPath::parse(string).map(|it| it.map0(Argument::MinecraftNbtPath))
            }
            Self::MinecraftObjective => {
                parse_minecraft_objective(string).map(|it| it.map0(Argument::MinecraftObjective))
            }
            Self::MinecraftObjectiveCriteria => parse_minecraft_objective_criteria(string)
                .map(|it| it.map0(Argument::MinecraftObjectiveCriteria)),
            Self::MinecraftOperation => {
                parse_minecraft_operation(string).map(|it| it.map0(Argument::MinecraftOperation))
            }
            Self::MinecraftResourceLocation => MinecraftResourceLocation::parse(string)
                .map(|it| it.map0(Argument::MinecraftResourceLocation)),
            Self::MinecraftRotation => {
                MinecraftRotation::parse(string).map(|it| it.map0(Argument::MinecraftRotation))
            }
            Self::MinecraftScoreHolder { .. } => parse_minecraft_score_holder(string)
                .map(|it| it.map0(Argument::MinecraftScoreHolder)),
            Self::MinecraftScoreboardSlot => parse_minecraft_scoreboard_slot(string)
                .map(|it| it.map0(Argument::MinecraftScoreboardSlot)),
            Self::MinecraftSwizzle => {
                parse_minecraft_swizzle(string).map(|it| it.map0(Argument::MinecraftSwizzle))
            }
            Self::MinecraftTime => {
                parse_minecraft_time(string).map(|it| it.map0(Argument::MinecraftTime))
            }
            ArgumentParser::MinecraftTeam => {
                parse_minecraft_team(string).map(|it| it.map0(Argument::MinecraftTeam))
            }
            Self::MinecraftVec3 => {
                MinecraftVec3::parse(string).map(|it| it.map0(Argument::MinecraftVec3))
            }
            Self::Unknown => parse_unknown(string).map(|it| it.map0(Argument::Unknown)),
            _ => Err(format!(
                "Unsupported argument type: {}",
                self.name().unwrap_or_default()
            )),
        }
    }
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

fn parse_minecraft_message(message: &str) -> Result<(MinecraftMessage, usize), String> {
    let mut index = 0;
    let mut selectors = Vec::new();
    while let Some(i) = &message[index..].find('@') {
        index += i;
        match MinecraftSelector::parse(&message[index..]) {
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
    Ok(brigadier::parse_unquoted_string(string))
}

fn parse_minecraft_objective_criteria(
    string: &str,
) -> Result<(MinecraftObjectiveCriteria, usize), String> {
    let len = string.find(' ').unwrap_or(string.len());
    Ok((&string[..len], len))
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

fn parse_minecraft_score_holder(string: &str) -> Result<(MinecraftScoreHolder, usize), String> {
    if string.starts_with('@') {
        let (selector, len) = MinecraftSelector::parse(string)?;
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

fn parse_minecraft_scoreboard_slot(
    string: &str,
) -> Result<(MinecraftScoreboardSlot, usize), String> {
    Ok(brigadier::parse_unquoted_string(string))
}

fn parse_minecraft_swizzle(string: &str) -> Result<(MinecraftSwizzle, usize), String> {
    let len = string
        .find(' ')
        .ok_or("Failed to parse swizzle".to_string())?;
    let swizzle = ();
    Ok((swizzle, len))
}

fn parse_minecraft_team(string: &str) -> Result<(MinecraftTeam, usize), String> {
    Ok(parse_unquoted_string(string))
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
