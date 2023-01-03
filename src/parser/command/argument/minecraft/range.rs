// mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any
// Minecraft mods.
//
// © Copyright (C) 2021-2023 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
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

use std::str::FromStr;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinecraftRange<N> {
    pub min: Option<N>,
    pub max: Option<N>,
}

impl<N: Clone + FromStr> MinecraftRange<N> {
    pub fn parse(string: &str) -> Result<(Self, usize), String> {
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

        fn parse_number<N: FromStr>(string: &str) -> Result<Option<N>, String> {
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
        let min = parse_number(min)?;

        let (max, len) = if let Some(suffix) = suffix.strip_prefix(SEPERATOR) {
            let max_len = number_len(suffix);
            let max = parse_number(&suffix[..max_len])?;
            (max, min_len + SEPERATOR.len() + max_len)
        } else {
            (min.clone(), min_len)
        };

        if min.is_none() && max.is_none() {
            Err(EMPTY.to_string())
        } else {
            Ok((MinecraftRange { min, max }, len))
        }
    }
}
