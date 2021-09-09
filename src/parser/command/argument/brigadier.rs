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

use serde::{Deserialize, Serialize};
use std::{fmt::Display, marker::PhantomData, str::FromStr};

pub fn parse_unquoted_string(string: &str) -> Result<(&str, usize), String> {
    let len = string
        .find(|c| !is_allowed_in_unquoted_string(c))
        .unwrap_or(string.len());
    Ok((&string[..len], len))
}

fn is_allowed_in_unquoted_string(c: char) -> bool {
    return c >= '0' && c <= '9'
        || c >= 'A' && c <= 'Z'
        || c >= 'a' && c <= 'z'
        || c == '+'
        || c == '-'
        || c == '.'
        || c == '_';
}

pub fn parse_double(string: &str) -> Result<(f64, usize), String> {
    parse_number(string).map_err(|e| e.to_string())
}

pub trait Number: FromStr {
    fn type_name() -> &'static str;
}

impl Number for i32 {
    fn type_name() -> &'static str {
        "integer"
    }
}

impl Number for f64 {
    fn type_name() -> &'static str {
        "double"
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ParseNumberError<'l, N: Number> {
    Empty(PhantomData<N>),
    Invalid(&'l str),
}

impl<N: Number> Display for ParseNumberError<'_, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseNumberError::Empty(_) => write!(f, "Expected {}", N::type_name()),
            ParseNumberError::Invalid(string) => {
                write!(f, "Invalid {} '{}'", N::type_name(), string)
            }
        }
    }
}

pub fn parse_number<N: Number>(string: &str) -> Result<(N, usize), ParseNumberError<N>> {
    let len = string
        .find(|c| !is_allowed_number(c))
        .unwrap_or(string.len());

    let number = &string[..len];
    if number.is_empty() {
        Err(ParseNumberError::Empty(PhantomData))
    } else {
        let number = number
            .parse()
            .map_err(|_| ParseNumberError::Invalid(number))?;
        Ok((number, len))
    }
}

fn is_allowed_number(c: char) -> bool {
    c >= '0' && c <= '9' || c == '.' || c == '-'
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BrigadierStringType {
    Greedy,
    Phrase,
    Word,
}

pub fn parse_string(string: &str, type_: BrigadierStringType) -> Result<(&str, usize), String> {
    match type_ {
        BrigadierStringType::Greedy => Ok((string, string.len())),
        BrigadierStringType::Phrase => {
            Err("Unsupported type 'phrase' for argument parser brigadier:string".to_string())
        }
        BrigadierStringType::Word => parse_unquoted_string(string),
    }
}
