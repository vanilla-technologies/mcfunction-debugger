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

use std::{cmp::Ordering, convert::TryFrom, fmt::Display, hash::Hash};

pub type ResourceLocation = ResourceLocationRef<String>;

#[derive(Clone, Debug)]
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
        let (path, namespace_len) = if let Some((namespace, path)) = string.split_once(':') {
            if !namespace.chars().all(is_valid_namespace_char) {
                return Err(InvalidResourceLocation::InvalidNamespace);
            }
            (path, namespace.len())
        } else {
            (string, usize::MAX)
        };

        if !path.chars().all(is_valid_path_char) {
            Err(InvalidResourceLocation::InvalidPath)
        } else {
            Ok(ResourceLocationRef {
                string,
                namespace_len,
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
        if self.namespace_len == usize::MAX {
            "minecraft"
        } else {
            &self.string.as_ref()[..self.namespace_len]
        }
    }

    pub fn path(&self) -> &str {
        if self.namespace_len == usize::MAX {
            self.string.as_ref()
        } else {
            &self.string.as_ref()[self.namespace_len + 1..]
        }
    }

    pub fn to_owned(&self) -> ResourceLocation {
        ResourceLocation {
            string: self.string.as_ref().to_owned(),
            namespace_len: self.namespace_len,
        }
    }

    pub fn mcfunction_path(&self) -> String {
        format!("{}/functions/{}.mcfunction", self.namespace(), self.path())
            .replace('/', &std::path::MAIN_SEPARATOR.to_string())
    }
}

impl ResourceLocation {
    pub fn to_ref(&self) -> ResourceLocationRef<&str> {
        ResourceLocationRef {
            string: &self.string,
            namespace_len: self.namespace_len,
        }
    }
}

impl<S: AsRef<str>> PartialEq for ResourceLocationRef<S> {
    fn eq(&self, other: &Self) -> bool {
        self.namespace() == other.namespace() && self.path() == other.path()
    }
}

impl<S: AsRef<str>> Eq for ResourceLocationRef<S> {}

impl<S: AsRef<str>> PartialOrd for ResourceLocationRef<S> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S: AsRef<str>> Ord for ResourceLocationRef<S> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.namespace()
            .cmp(other.namespace())
            .then_with(|| self.path().cmp(other.path()))
    }
}

impl<S: AsRef<str>> Hash for ResourceLocationRef<S> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.namespace().hash(state);
        self.path().hash(state);
    }
}

impl<'l> ResourceLocationRef<&'l str> {
    pub fn parse(string: &'l str) -> Result<(Self, usize), String> {
        const INVALID_ID: &str = "Invalid ID";

        let len = string
            .find(|c| !Self::is_allowed_in_resource_location(c))
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
}

impl<S: AsRef<str>> Display for ResourceLocationRef<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.string.as_ref().fmt(f)
    }
}
