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

use crate::parser::command::argument::brigadier::{
    expect, is_quote, parse_int, parse_possibly_quoted_string, parse_quoted_string,
    parse_unquoted_string,
};
use ::nbt::{Map, Value};
use std::convert::TryFrom;

#[derive(Clone, Debug, PartialEq)]
pub struct MinecraftNbtPath<'l>(Vec<MinecraftNbtPathNode<'l>>);

impl<'l> MinecraftNbtPath<'l> {
    pub fn parse(string: &'l str) -> Result<(Self, usize), String> {
        let mut path = Vec::new();
        let mut root = true;
        let mut suffix = string;
        while !suffix.is_empty() && !suffix.starts_with(' ') {
            let (node, len) = MinecraftNbtPathNode::parse(suffix, root)?;
            path.push(node);
            suffix = &suffix[len..];
            root = false;
            if !matches!(suffix.chars().next(), Some(' ' | '[' | '{') | None) {
                suffix = expect(suffix, '.')?;
            }
        }

        Ok((MinecraftNbtPath(path), string.len() - suffix.len()))
    }
}

#[derive(Clone, Debug, PartialEq)]
enum MinecraftNbtPathNode<'l> {
    AllElements,
    CompoundChild(&'l str),
    IndexedElement(i32),
    MatchElement(CompoundNbt),
    MatchObject(&'l str, CompoundNbt),
    MatchRootObject(CompoundNbt),
}

impl<'l> MinecraftNbtPathNode<'l> {
    fn parse(string: &'l str, root: bool) -> Result<(Self, usize), String> {
        let c = string
            .chars()
            .next()
            .ok_or("Expected nbt path".to_string())?;

        match c {
            '"' => {
                let (name, name_len) = parse_quoted_string(&string, '"')?;
                let suffix = &string[name_len..];
                parse_object_node(suffix, name, name_len)
            }
            '[' => {
                let mut suffix = &string['['.len_utf8()..];
                if suffix.starts_with('{') {
                    let (compound, len) = CompoundNbt::parse(suffix)?;
                    suffix = &suffix[len..];
                    suffix = expect(suffix, ']')?;
                    Ok((Self::MatchElement(compound), string.len() - suffix.len()))
                } else if suffix.starts_with(']') {
                    Ok((Self::AllElements, "[]".len()))
                } else {
                    let (index, len) = parse_int(suffix)?;
                    suffix = &suffix[len..];
                    suffix = expect(suffix, ']')?;
                    Ok((Self::IndexedElement(index), string.len() - suffix.len()))
                }
            }
            '{' => {
                if root {
                    let (compound, len) = CompoundNbt::parse(string)?;
                    Ok((Self::MatchRootObject(compound), len))
                } else {
                    Err(INVALID_NODE.to_string())
                }
            }
            _ => {
                let (name, name_len) = parse_unquoted_name(&string)?;
                let suffix = &string[name_len..];
                parse_object_node(suffix, name, name_len)
            }
        }
    }
}

fn parse_object_node<'l>(
    suffix: &str,
    name: &'l str,
    name_len: usize,
) -> Result<(MinecraftNbtPathNode<'l>, usize), String> {
    use MinecraftNbtPathNode::*;
    if suffix.starts_with("{") {
        let (compound, compound_len) = CompoundNbt::parse(suffix)?;
        Ok((MatchObject(name, compound), name_len + compound_len))
    } else {
        Ok((CompoundChild(name), name_len))
    }
}

const INVALID_NODE: &str = "Invalid NBT path element";

pub fn parse_unquoted_name(string: &str) -> Result<(&str, usize), String> {
    let len = string
        .find(|c| !is_allowed_in_unquoted_name(c))
        .unwrap_or(string.len());
    if len != 0 {
        Ok((&string[..len], len))
    } else {
        Err(INVALID_NODE.to_string())
    }
}

fn is_allowed_in_unquoted_name(c: char) -> bool {
    c != ' ' && c != '"' && c != '[' && c != ']' && c != '.' && c != '{' && c != '}'
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompoundNbt(pub Map<String, Value>);

impl CompoundNbt {
    pub fn parse(string: &str) -> Result<(Self, usize), String> {
        let mut compound = Map::new();

        let mut suffix = expect(string, '{')?.trim_start();
        if !suffix.is_empty() {
            while !suffix.starts_with('}') {
                let (key, len) = parse_key(suffix)?;
                suffix = &suffix[len..].trim_start();

                suffix = expect(suffix, ':')?.trim_start();

                let (value, len) = parse_value(suffix)?;
                suffix = &suffix[len..].trim_start();

                compound.insert(key, value);

                if let Some(s) = suffix.strip_prefix(',') {
                    suffix = s.trim_start();
                } else {
                    break;
                }

                if suffix.is_empty() {
                    return Err(EXPECTED_KEY.to_string());
                }
            }
        }
        suffix = expect(suffix, '}')?;

        Ok((CompoundNbt(compound), string.len() - suffix.len()))
    }
}

const EXPECTED_KEY: &str = "Expected key";
const EXPECTED_VALUE: &str = "Expected value";

fn parse_key(string: &str) -> Result<(String, usize), String> {
    let (string, len) = parse_possibly_quoted_string(string)?;
    if string.is_empty() {
        Err(EXPECTED_KEY.to_string())
    } else {
        Ok((string.to_string(), len))
    }
}

fn parse_value(string: &str) -> Result<(Value, usize), String> {
    let c = string.chars().next();
    if let Some(c) = c {
        match c {
            '{' => {
                let (CompoundNbt(compound), len) = CompoundNbt::parse(string)?;
                Ok((Value::Compound(compound), len))
            }
            '[' => parse_array_or_list(string),
            quote if is_quote(quote) => {
                let (string, len) = parse_quoted_string(string, quote)?;
                Ok((Value::String(string.to_string()), len))
            }
            _ => {
                let (string, len) = parse_unquoted_string(string);
                // TODO parse other value types from string
                Ok((Value::String(string.to_string()), len))
            }
        }
    } else {
        Err(EXPECTED_VALUE.to_string())
    }
}

fn parse_array_or_list(string: &str) -> Result<(Value, usize), String> {
    let mut suffix = expect(string, '[')?;
    let mut chars = string.chars();
    if let (Some(array_type), Some(';')) = (chars.next(), chars.next()) {
        suffix = &suffix[array_type.len_utf8() + ';'.len_utf8()..].trim_start();
        let (value, len) = match array_type {
            'B' => ByteArrayParser.parse_suffix(suffix),
            'L' => LongArrayParser.parse_suffix(suffix),
            'I' => IntArrayParser.parse_suffix(suffix),
            value => Err(format!("Invalid array type '{}'", value)),
        }?;
        Ok((value, (string.len() - suffix.len()) + len))
    } else {
        let (list, len) = parse_list(string)?;
        Ok((Value::List(list), len))
    }
}

fn parse_list(string: &str) -> Result<(Vec<Value>, usize), String> {
    let mut vec = Vec::new();

    let mut suffix = expect(string, '[')?.trim_start();
    while !suffix.starts_with(']') {
        let (value, len) = parse_value(suffix)?;
        suffix = &suffix[len..].trim_start();

        vec.push(value);

        if let Some(s) = suffix.strip_prefix(',') {
            suffix = s.trim_start();
        } else {
            break;
        }

        if suffix.is_empty() {
            return Err(EXPECTED_VALUE.to_string());
        }
    }
    suffix = expect(suffix, ']')?;
    Ok((vec, string.len() - suffix.len()))
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum NbtArrayType {
    Byte,
    Long,
    Int,
}

impl TryFrom<char> for NbtArrayType {
    type Error = String;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'B' => Ok(NbtArrayType::Byte),
            'L' => Ok(NbtArrayType::Long),
            'I' => Ok(NbtArrayType::Int),
            value => Err(format!("Invalid array type '{}'", value)),
        }
    }
}

trait NbtArrayParser<E> {
    fn parse_suffix(&self, string: &str) -> Result<(Value, usize), String> {
        let mut vec = Vec::new();
        let mut suffix = string;
        while !suffix.starts_with(']') {
            let (value, len) = parse_value(suffix)?;
            suffix = &suffix[len..].trim_start();

            let tag_name = value.tag_name().to_string();
            let element = self.to_element(value).ok_or(format!(
                "Can't insert {} into {}",
                tag_name,
                self.tag_name()
            ))?;
            vec.push(element);

            if let Some(s) = suffix.strip_prefix(',') {
                suffix = s.trim_start();
            } else {
                break;
            }

            if suffix.is_empty() {
                return Err(EXPECTED_VALUE.to_string());
            }
        }
        suffix = expect(suffix, ']')?;

        Ok((self.to_value(vec), string.len() - suffix.len()))
    }

    fn tag_name(&self) -> String {
        self.to_value(Vec::new()).tag_name().to_string()
    }

    fn to_element(&self, value: Value) -> Option<E>;

    fn to_value(&self, vec: Vec<E>) -> Value;
}

struct ByteArrayParser;

impl NbtArrayParser<i8> for ByteArrayParser {
    fn to_element(&self, value: Value) -> Option<i8> {
        if let Value::Byte(byte) = value {
            Some(byte)
        } else {
            None
        }
    }

    fn to_value(&self, vec: Vec<i8>) -> Value {
        Value::ByteArray(vec)
    }
}

struct IntArrayParser;

impl NbtArrayParser<i32> for IntArrayParser {
    fn to_element(&self, value: Value) -> Option<i32> {
        if let Value::Int(int) = value {
            Some(int)
        } else {
            None
        }
    }

    fn to_value(&self, vec: Vec<i32>) -> Value {
        Value::IntArray(vec)
    }
}

struct LongArrayParser;

impl NbtArrayParser<i64> for LongArrayParser {
    fn to_element(&self, value: Value) -> Option<i64> {
        if let Value::Long(long) = value {
            Some(long)
        } else {
            None
        }
    }

    fn to_value(&self, vec: Vec<i64>) -> Value {
        Value::LongArray(vec)
    }
}
