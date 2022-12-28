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

use serde::{Deserialize, Serialize};
use std::{fmt::Display, marker::PhantomData, str::FromStr};

pub fn expect(string: &str, prefix: char) -> Result<&str, String> {
    string
        .strip_prefix(prefix)
        .ok_or(format!("Expected '{}'", prefix))
}

pub fn parse_possibly_quoted_string(string: &str) -> Result<(&str, usize), String> {
    if let Some(quote) = string.chars().next() {
        if is_quote(quote) {
            parse_quoted_string(string, quote)
        } else {
            Ok(parse_unquoted_string(string))
        }
    } else {
        Ok(("", 0))
    }
}

pub fn parse_quoted_string(string: &str, quote: char) -> Result<(&str, usize), String> {
    let suffix = &string[quote.len_utf8()..];
    let (string, len) = parse_string_until(suffix, quote)?;
    Ok((string, quote.len_utf8() + len))
}

pub fn is_quote(c: char) -> bool {
    c == '"' || c == '\''
}

fn parse_string_until(string: &str, terminator: char) -> Result<(&str, usize), String> {
    let index = find_unescaped(string, terminator).ok_or("Unclosed quoted string".to_string())?;
    Ok((&string[..index], index + terminator.len_utf8()))
}

fn find_unescaped(string: &str, to_find: char) -> Option<usize> {
    const ESCAPE: char = '\\';
    string
        .char_indices()
        // Mark escaped chars
        .scan(false, |escaped, (index, c)| {
            let e = *escaped;
            if e {
                *escaped = false;
            } else if c == ESCAPE {
                *escaped = true;
            }
            Some((index, c, e))
        })
        // Filter for unescaped chars
        .filter(|(_index, _c, escaped)| !escaped)
        .find(|(_index, c, _)| *c == to_find)
        .map(|(index, ..)| index)
}

pub fn parse_unquoted_string(string: &str) -> (&str, usize) {
    let len = string
        .find(|c| !is_allowed_in_unquoted_string(c))
        .unwrap_or(string.len());
    (&string[..len], len)
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

pub fn parse_bool(string: &str) -> Result<(bool, usize), String> {
    let (string, len) = parse_unquoted_string(string);
    match string {
        "false" => Ok((false, len)),
        "true" => Ok((true, len)),
        string => Err(format!(
            "Invalid bool, expected true or false but found '{}'",
            string
        )),
    }
}

pub fn parse_double(string: &str) -> Result<(f64, usize), String> {
    parse_number(string).map_err(|e| e.to_string())
}

pub fn parse_integer(string: &str) -> Result<(i32, usize), String> {
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
        BrigadierStringType::Word => Ok(parse_unquoted_string(string)),
    }
}
