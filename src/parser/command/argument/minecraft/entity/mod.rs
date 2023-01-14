// McFunction-Debugger is a debugger for Minecraft's *.mcfunction files that does not require any
// Minecraft mods.
//
// © Copyright (C) 2021-2023 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
//
// This file is part of McFunction-Debugger.
//
// McFunction-Debugger is free software: you can redistribute it and/or modify it under the terms of
// the GNU General Public License as published by the Free Software Foundation, either version 3 of
// the License, or (at your option) any later version.
//
// McFunction-Debugger is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with McFunction-Debugger.
// If not, see <http://www.gnu.org/licenses/>.

use crate::{
    parser::command::{
        argument::{
            brigadier::{
                expect, parse_bool, parse_double, parse_integer, parse_possibly_quoted_string,
                parse_unquoted_string,
            },
            minecraft::{nbt::CompoundNbt, range::MinecraftRange},
        },
        resource_location::ResourceLocationRef,
    },
    utils::Map0,
};
use log::warn;
use std::{collections::BTreeMap, fmt::Display};

#[derive(Clone, Debug, PartialEq)]
pub enum MinecraftEntity<'l> {
    Selector(MinecraftSelector<'l>),
    PlayerNameOrUuid(&'l str),
}

impl<'l> MinecraftEntity<'l> {
    pub fn parse(string: &'l str) -> Result<(Self, usize), String> {
        if string.starts_with('@') {
            MinecraftSelector::parse(string)
                .map(|it| it.map0(MinecraftEntity::Selector))
                .map_err(Into::into)
        } else {
            parse_possibly_quoted_string(string)
                .map(|it| it.map0(MinecraftEntity::PlayerNameOrUuid))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MinecraftSelectorParserError {
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

#[derive(Clone, Debug, PartialEq)]
pub struct MinecraftSelector<'l> {
    selector_type: MinecraftSelectorType,
    name: Option<InvertableString<'l>>,
    distance: Option<MinecraftRange<f64>>,
    level: Option<MinecraftRange<i32>>,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
    dx: Option<f64>,
    dy: Option<f64>,
    dz: Option<f64>,
    x_rotation: Option<MinecraftRange<f32>>,
    y_rotation: Option<MinecraftRange<f32>>,
    limit: Option<i32>,
    sort: Option<&'l str>,
    gamemode: Option<InvertableString<'l>>,
    team: Option<InvertableString<'l>>,
    entity_type: Option<EntityType<'l>>,
    tags: Vec<InvertableString<'l>>,
    nbts: Vec<InvertableCompoundNbt>,
    scores: BTreeMap<&'l str, MinecraftRange<i32>>,
    advancements: BTreeMap<ResourceLocationRef<&'l str>, MinecraftAdvancementProgress<'l>>,
    predicates: Vec<InvertablePredicate<'l>>,
}

impl<'l> MinecraftSelector<'l> {
    pub fn parse(string: &'l str) -> Result<(Self, usize), MinecraftSelectorParserError> {
        type Error = MinecraftSelectorParserError;

        let mut suffix = string
            .strip_prefix('@')
            .ok_or(Error::Other(format!("Invalid selector {}", string)))?;

        let (selector_type, len) = MinecraftSelectorType::parse(suffix)?;
        suffix = &suffix[len..];

        let mut selector = MinecraftSelector::new(selector_type);

        if let Some(s) = suffix.strip_prefix('[') {
            suffix = s.trim_start();

            while !suffix.is_empty() && !suffix.starts_with(']') {
                let (key, len) = parse_possibly_quoted_string(suffix).map_err(Error::Other)?;
                suffix = &suffix[len..].trim_start();

                suffix = expect(suffix, '=').map_err(Error::Other)?.trim_start();

                let len = selector
                    .parse_option_value(key, suffix)
                    .map_err(Error::Other)?;
                suffix = &suffix[len..].trim_start();

                if let Some(s) = suffix.strip_prefix(',') {
                    suffix = s.trim_start();
                } else {
                    break;
                }
            }
            suffix = expect(suffix, ']').map_err(Error::Other)?;
        }
        Ok((selector, string.len() - suffix.len()))
    }

    fn new(selector_type: MinecraftSelectorType) -> MinecraftSelector<'l> {
        MinecraftSelector {
            selector_type,
            name: None,
            distance: None,
            level: None,
            x: None,
            y: None,
            z: None,
            dx: None,
            dy: None,
            dz: None,
            x_rotation: None,
            y_rotation: None,
            limit: None,
            sort: None,
            gamemode: None,
            team: None,
            entity_type: None,
            tags: Vec::new(),
            nbts: Vec::new(),
            scores: BTreeMap::new(),
            advancements: BTreeMap::new(),
            predicates: Vec::new(),
        }
    }

    fn parse_option_value(&mut self, key: &str, string: &'l str) -> Result<usize, String> {
        match key {
            "name" => {
                let (name, len) = InvertableString::parse_possibly_quoted(string)?;
                self.name = Some(name);
                Ok(len)
            }
            "distance" => {
                let (distance, len) = MinecraftRange::parse(string)?;
                self.distance = Some(distance);
                Ok(len)
            }
            "level" => {
                let (level, len) = MinecraftRange::parse(string)?;
                self.level = Some(level);
                Ok(len)
            }
            "x" => {
                let (x, len) = parse_double(string)?;
                self.x = Some(x);
                Ok(len)
            }
            "y" => {
                let (y, len) = parse_double(string)?;
                self.y = Some(y);
                Ok(len)
            }
            "z" => {
                let (z, len) = parse_double(string)?;
                self.z = Some(z);
                Ok(len)
            }
            "dx" => {
                let (dx, len) = parse_double(string)?;
                self.dx = Some(dx);
                Ok(len)
            }
            "dy" => {
                let (dy, len) = parse_double(string)?;
                self.dy = Some(dy);
                Ok(len)
            }
            "dz" => {
                let (dz, len) = parse_double(string)?;
                self.dz = Some(dz);
                Ok(len)
            }
            "x_rotation" => {
                let (x_rotation, len) = MinecraftRange::parse(string)?;
                self.x_rotation = Some(x_rotation);
                Ok(len)
            }
            "y_rotation" => {
                let (y_rotation, len) = MinecraftRange::parse(string)?;
                self.y_rotation = Some(y_rotation);
                Ok(len)
            }
            "limit" => {
                let (limit, len) = parse_integer(string)?;
                self.limit = Some(limit);
                Ok(len)
            }
            "sort" => {
                let (sort, len) = parse_unquoted_string(string);
                self.sort = Some(sort);
                Ok(len)
            }
            "gamemode" => {
                let (gamemode, len) = InvertableString::parse_unquoted(string);
                self.gamemode = Some(gamemode);
                Ok(len)
            }
            "team" => {
                let (team, len) = InvertableString::parse_unquoted(string);
                self.team = Some(team);
                Ok(len)
            }
            "type" => {
                let (entity_type, len) = EntityType::parse(string)?;
                self.entity_type = Some(entity_type);
                Ok(len)
            }
            "tag" => {
                let (tag, len) = InvertableString::parse_unquoted(string);
                self.tags.push(tag);
                Ok(len)
            }
            "nbt" => {
                let (nbt, len) = InvertableCompoundNbt::parse(string)?;
                self.nbts.push(nbt);
                Ok(len)
            }
            "scores" => {
                let (scores, len) = parse_scores(string)?;
                self.scores = scores;
                Ok(len)
            }
            "advancements" => {
                let (advancements, len) = parse_advancements(string)?;
                self.advancements = advancements;
                Ok(len)
            }
            "predicate" => {
                let (predicate, len) = InvertablePredicate::parse(string)?;
                self.predicates.push(predicate);
                Ok(len)
            }
            _ => {
                warn!("Unknown option '{}'", key);
                let len = string.find(&[',', ']'][..]).unwrap_or(string.len());
                Ok(len)
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MinecraftSelectorType {
    A,
    E,
    P,
    R,
    S,
}

impl MinecraftSelectorType {
    pub fn parse(string: &str) -> Result<(Self, usize), MinecraftSelectorParserError> {
        let c = string
            .chars()
            .next()
            .ok_or(MinecraftSelectorParserError::MissingSelectorType)?;
        match c {
            'a' => Ok((Self::A, c.len_utf8())),
            'e' => Ok((Self::E, c.len_utf8())),
            'p' => Ok((Self::P, c.len_utf8())),
            'r' => Ok((Self::R, c.len_utf8())),
            's' => Ok((Self::S, c.len_utf8())),
            c => Err(MinecraftSelectorParserError::UnknownSelectorType(c)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvertableString<'l> {
    inverted: bool,
    string: &'l str,
}

impl<'l> InvertableString<'l> {
    fn parse_possibly_quoted(string: &'l str) -> Result<(Self, usize), String> {
        let (inverted, suffix) = parse_prefix(string, '!');
        let (value, len) = parse_possibly_quoted_string(suffix)?;
        let suffix = &suffix[len..];
        Ok((
            InvertableString {
                inverted,
                string: value,
            },
            string.len() - suffix.len(),
        ))
    }

    fn parse_unquoted(string: &'l str) -> (Self, usize) {
        let (inverted, suffix) = parse_prefix(string, '!');
        let (value, len) = parse_unquoted_string(suffix);
        let suffix = &suffix[len..];
        (
            InvertableString {
                inverted,
                string: value,
            },
            string.len() - suffix.len(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityType<'l> {
    inverted: bool,
    tag: bool,
    resource_location: ResourceLocationRef<&'l str>,
}

impl<'l> EntityType<'l> {
    fn parse(string: &'l str) -> Result<(Self, usize), String> {
        let (inverted, suffix) = parse_prefix(string, '!');
        let (tag, suffix) = parse_prefix(suffix, '#');
        let (resource_location, len) = ResourceLocationRef::parse(suffix)?;
        let suffix = &suffix[len..];
        Ok((
            EntityType {
                inverted,
                tag,
                resource_location,
            },
            string.len() - suffix.len(),
        ))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvertableCompoundNbt {
    inverted: bool,
    nbt: CompoundNbt,
}

impl InvertableCompoundNbt {
    fn parse(string: &str) -> Result<(Self, usize), String> {
        let (inverted, suffix) = parse_prefix(string, '!');
        let (nbt, len) = CompoundNbt::parse(suffix)?;
        let suffix = &suffix[len..];
        Ok((
            InvertableCompoundNbt { inverted, nbt },
            string.len() - suffix.len(),
        ))
    }
}

fn parse_prefix(string: &str, prefix: char) -> (bool, &str) {
    let suffix = string.strip_prefix(prefix);
    (suffix.is_some(), suffix.unwrap_or(string).trim_start())
}

fn parse_scores(string: &str) -> Result<(BTreeMap<&str, MinecraftRange<i32>>, usize), String> {
    let mut scores = BTreeMap::new();

    let mut suffix = expect(string, '{')?.trim_start();
    while !suffix.is_empty() && !suffix.starts_with('}') {
        let (key, len) = parse_unquoted_string(suffix);
        suffix = &suffix[len..].trim_start();

        suffix = expect(suffix, '=')?.trim_start();

        let (value, len) = MinecraftRange::parse(suffix)?;
        suffix = &suffix[len..].trim_start();

        scores.insert(key, value);

        if let Some(s) = suffix.strip_prefix(',') {
            suffix = s.trim_start();
        }
    }
    suffix = expect(suffix, '}')?;

    Ok((scores, string.len() - suffix.len()))
}

fn parse_advancements(
    string: &str,
) -> Result<
    (
        BTreeMap<ResourceLocationRef<&str>, MinecraftAdvancementProgress>,
        usize,
    ),
    String,
> {
    let mut advancements = BTreeMap::new();

    let mut suffix = expect(string, '{')?.trim_start();
    while !suffix.is_empty() && !suffix.starts_with('}') {
        let (key, len) = ResourceLocationRef::parse(suffix)?;
        suffix = &suffix[len..].trim_start();

        suffix = expect(suffix, '=')?.trim_start();

        let (value, len) = MinecraftAdvancementProgress::parse(suffix)?;
        suffix = &suffix[len..].trim_start();

        advancements.insert(key, value);

        if let Some(s) = suffix.strip_prefix(',') {
            suffix = s.trim_start();
        }
    }
    suffix = expect(suffix, '}')?;

    Ok((advancements, string.len() - suffix.len()))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MinecraftAdvancementProgress<'l> {
    AdvancementProgress(bool),
    CriterionProgress(BTreeMap<&'l str, bool>),
}

impl<'l> MinecraftAdvancementProgress<'l> {
    fn parse(string: &'l str) -> Result<(Self, usize), String> {
        let mut suffix = string;
        let progress = if let Some(s) = suffix.strip_prefix('{') {
            suffix = s.trim_start();

            let mut criteria = BTreeMap::new();

            while !suffix.is_empty() && !suffix.starts_with('}') {
                let (criterion, len) = parse_unquoted_string(suffix);
                suffix = &suffix[len..].trim_start();

                suffix = expect(suffix, '=')?.trim_start();

                let (b, len) = parse_bool(suffix)?;
                suffix = &suffix[len..].trim_start();

                criteria.insert(criterion, b);

                if let Some(s) = suffix.strip_prefix(',') {
                    suffix = s.trim_start();
                }
            }
            suffix = expect(suffix, '}')?;

            Self::CriterionProgress(criteria)
        } else {
            let (b, len) = parse_bool(suffix)?;
            suffix = &suffix[len..].trim_start();

            Self::AdvancementProgress(b)
        };
        Ok((progress, string.len() - suffix.len()))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvertablePredicate<'l> {
    inverted: bool,
    predicate: ResourceLocationRef<&'l str>,
}

impl<'l> InvertablePredicate<'l> {
    fn parse(string: &'l str) -> Result<(Self, usize), String> {
        let (inverted, suffix) = parse_prefix(string, '!');
        let (predicate, len) = ResourceLocationRef::parse(suffix)?;
        let suffix = &suffix[len..];
        Ok((
            InvertablePredicate {
                inverted,
                predicate,
            },
            string.len() - suffix.len(),
        ))
    }
}

#[cfg(test)]
mod tests;
