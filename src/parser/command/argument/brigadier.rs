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
use std::str::FromStr;

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

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ParseNumberError<'l> {
    Empty,
    Invalid(&'l str),
}

pub fn parse_number<N: FromStr>(string: &str) -> Result<(N, usize), ParseNumberError> {
    let len = string
        .find(|c| !is_allowed_number(c))
        .unwrap_or(string.len());

    let number = &string[..len];
    if number.is_empty() {
        Err(ParseNumberError::Empty)
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
