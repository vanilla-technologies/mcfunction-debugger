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

use std::{convert::TryFrom, fmt::Display};

pub type ResourceLocation = ResourceLocationRef<String>;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ResourceLocationRef<S: AsRef<str>> {
    string: S,
    namespace_len: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InvalidResourceLocation {
    InvalidNamespace,
    InvalidPath,
}

impl<'l> TryFrom<&'l str> for ResourceLocationRef<&'l str> {
    type Error = InvalidResourceLocation;

    fn try_from(string: &'l str) -> Result<Self, Self::Error> {
        let (namespace, path) = string.split_once(':').unwrap_or(("minecraft", string));

        if !namespace.chars().all(is_valid_namespace_char) {
            Err(InvalidResourceLocation::InvalidNamespace)
        } else if !path.chars().all(is_valid_path_char) {
            Err(InvalidResourceLocation::InvalidPath)
        } else {
            Ok(ResourceLocationRef {
                string,
                namespace_len: namespace.len(),
            })
        }
    }
}

fn is_valid_namespace_char(c: char) -> bool {
    c >= '0' && c <= '9' || c >= 'a' && c <= 'z' || c == '-' || c == '.' || c == '_'
}

fn is_valid_path_char(c: char) -> bool {
    c >= '0' && c <= '9' || c >= 'a' && c <= 'z' || c == '-' || c == '.' || c == '/' || c == '_'
}

impl<S: AsRef<str>> ResourceLocationRef<S> {
    pub fn new(namespace: &str, path: &str) -> ResourceLocation {
        ResourceLocationRef {
            string: format!("{}:{}", namespace, path),
            namespace_len: namespace.len(),
        }
    }

    pub fn namespace(&self) -> &str {
        &self.string.as_ref()[..self.namespace_len]
    }

    pub fn path(&self) -> &str {
        &self.string.as_ref()[self.namespace_len + 1..]
    }

    pub fn to_owned(&self) -> ResourceLocation {
        ResourceLocation {
            string: self.string.as_ref().to_owned(),
            namespace_len: self.namespace_len,
        }
    }
}

impl<S: AsRef<str>> Display for ResourceLocationRef<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.string.as_ref().fmt(f)
    }
}
