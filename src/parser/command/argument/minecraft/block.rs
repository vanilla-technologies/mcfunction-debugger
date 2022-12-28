// mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any
// Minecraft mods.
//
// © Copyright (C) 2021, 2022 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
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

use crate::parser::command::{
    argument::{
        brigadier::{expect, parse_possibly_quoted_string},
        minecraft::nbt::CompoundNbt,
    },
    resource_location::ResourceLocationRef,
};
use ::nbt::Map;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub struct MinecraftBlockPredicate<'l> {
    tag: bool,
    block: ResourceLocationRef<&'l str>,
    properties: BTreeMap<&'l str, &'l str>,
    nbt: CompoundNbt,
}

impl<'l> MinecraftBlockPredicate<'l> {
    pub fn parse(string: &'l str) -> Result<(Self, usize), String> {
        let suffix = string;

        let tag = string.starts_with('#');
        let tag_len = if tag { '#'.len_utf8() } else { 0 };
        let suffix = &suffix[tag_len..];

        let (block, block_len) = ResourceLocationRef::parse(suffix)?;
        let suffix = &suffix[block_len..];

        let (properties, properties_len) = parse_properties(suffix)?;
        let suffix = &suffix[properties_len..];

        let (nbt, nbt_len) = parse_nbt(suffix)?;

        Ok((
            MinecraftBlockPredicate {
                tag,
                block,
                properties,
                nbt,
            },
            tag_len + block_len + properties_len + nbt_len,
        ))
    }
}

fn parse_properties(string: &str) -> Result<(BTreeMap<&str, &str>, usize), String> {
    let mut properties = BTreeMap::new();
    let mut suffix = string;
    if let Some(s) = string.strip_prefix('[') {
        suffix = s.trim_start();
        while !suffix.starts_with(']') {
            let (key, len) = parse_possibly_quoted_string(suffix)?;
            suffix = &suffix[len..].trim_start();

            suffix = expect(suffix, '=')?.trim_start();

            let (value, len) = parse_possibly_quoted_string(suffix)?;
            suffix = &suffix[len..].trim_start();

            properties.insert(key, value);

            if let Some(s) = suffix.strip_prefix(',') {
                suffix = s.trim_start();
            } else {
                break;
            }
        }
        suffix = expect(suffix, ']')?;
    }
    Ok((properties, string.len() - suffix.len()))
}

fn parse_nbt(string: &str) -> Result<(CompoundNbt, usize), String> {
    if string.starts_with('{') {
        CompoundNbt::parse(string)
    } else {
        Ok((CompoundNbt(Map::new()), 0))
    }
}
