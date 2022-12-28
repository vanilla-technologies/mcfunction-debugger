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

use crate::parser::command::argument::brigadier::{self, parse_number, ParseNumberError};

const INCOMPLETE_2: &str = "Incomplete (expected 2 coordinates)";
const INCOMPLETE_3: &str = "Incomplete (expected 3 coordinates)";
const CANNOT_MIX: &str =
    "Cannot mix world & local coordinates (everyhing must either use ^ or not)";

#[derive(Clone, Debug, PartialEq)]
pub struct MinecraftBlockPos(pub MinecraftVec3);

impl MinecraftBlockPos {
    pub fn parse(string: &str) -> Result<(Self, usize), String> {
        if string.starts_with('^') {
            let (argument, len) = LocalCoordinates::parse(string)?;
            Ok((MinecraftBlockPos(MinecraftVec3::Local(argument)), len))
        } else {
            let (argument, len) = WorldCoordinates::parse_int(string)?;
            Ok((MinecraftBlockPos(MinecraftVec3::World(argument)), len))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MinecraftRotation {
    x_relative: bool,
    x: f64,
    y_relative: bool,
    y: f64,
}

impl MinecraftRotation {
    pub fn parse(string: &str) -> Result<(Self, usize), String> {
        let suffix = string;
        check_non_local(suffix)?;
        let (x_relative, len) = parse_relative(suffix);
        let suffix = &suffix[len..];
        let (x, len) = parse_number_or_default(suffix)?;
        let suffix = &suffix[len..];
        let suffix = suffix.strip_prefix(' ').ok_or(INCOMPLETE_2.to_string())?;
        check_non_local(suffix)?;
        let (y_relative, len) = parse_relative(suffix);
        let suffix = &suffix[len..];
        let (y, len) = parse_number_or_default(suffix)?;
        let suffix = &suffix[len..];
        let rotation = MinecraftRotation {
            x_relative,
            x,
            y_relative,
            y,
        };
        let len = string.len() - suffix.len();
        Ok((rotation, len))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MinecraftVec3 {
    Local(LocalCoordinates),
    World(WorldCoordinates),
}

impl MinecraftVec3 {
    pub fn parse(string: &str) -> Result<(Self, usize), String> {
        if string.starts_with('^') {
            let (argument, len) = LocalCoordinates::parse(string)?;
            Ok((MinecraftVec3::Local(argument), len))
        } else {
            let (argument, len) = WorldCoordinates::parse_double(string)?;
            Ok((MinecraftVec3::World(argument), len))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorldCoordinates {
    x_relative: bool,
    x: f64,
    y_relative: bool,
    y: f64,
    z_relative: bool,
    z: f64,
}

impl WorldCoordinates {
    fn parse_double(string: &str) -> Result<(Self, usize), String> {
        WorldCoordinates::parse::<f64>(string)
    }

    fn parse_int(string: &str) -> Result<(Self, usize), String> {
        WorldCoordinates::parse::<i32>(string)
    }

    fn parse<N: Number>(string: &str) -> Result<(Self, usize), String> {
        let suffix = string;
        let (x_relative, len) = parse_relative(suffix);
        let suffix = &suffix[len..];
        let (x, len) = WorldCoordinates::parse_coordinate::<N>(suffix, x_relative)?;
        let suffix = &suffix[len..];
        let suffix = suffix.strip_prefix(' ').ok_or(INCOMPLETE_3.to_string())?;
        check_non_local(suffix)?;
        let (y_relative, len) = parse_relative(suffix);
        let suffix = &suffix[len..];
        let (y, len) = WorldCoordinates::parse_coordinate::<N>(suffix, y_relative)?;
        let suffix = &suffix[len..];
        let suffix = suffix.strip_prefix(' ').ok_or(INCOMPLETE_3.to_string())?;
        check_non_local(suffix)?;
        let (z_relative, len) = parse_relative(suffix);
        let suffix = &suffix[len..];
        let (z, len) = WorldCoordinates::parse_coordinate::<N>(suffix, z_relative)?;
        let suffix = &suffix[len..];
        let coordinates = WorldCoordinates {
            x_relative,
            x,
            y_relative,
            y,
            z_relative,
            z,
        };
        let len = string.len() - suffix.len();
        Ok((coordinates, len))
    }

    fn parse_coordinate<N: Number>(string: &str, relative: bool) -> Result<(f64, usize), String> {
        if relative {
            parse_number_or_default(string)
        } else {
            let (number, len) = parse_number_or_default::<N>(string)?;
            Ok((number.into(), len))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LocalCoordinates {
    x: f64,
    y: f64,
    z: f64,
}

impl LocalCoordinates {
    fn parse(string: &str) -> Result<(Self, usize), String> {
        let suffix = string.strip_prefix('^').ok_or(CANNOT_MIX.to_string())?;
        let (x, len) = parse_number_or_default(suffix)?;
        let suffix = &suffix[len..];
        let suffix = suffix.strip_prefix(' ').ok_or(INCOMPLETE_3.to_string())?;
        let suffix = suffix.strip_prefix('^').ok_or(CANNOT_MIX.to_string())?;
        let (y, len) = parse_number_or_default(suffix)?;
        let suffix = &suffix[len..];
        let suffix = suffix.strip_prefix(' ').ok_or(INCOMPLETE_3.to_string())?;
        let suffix = suffix.strip_prefix('^').ok_or(CANNOT_MIX.to_string())?;
        let (z, len) = parse_number_or_default(suffix)?;
        let suffix = &suffix[len..];
        let coordinates = LocalCoordinates { x, y, z };
        let len = string.len() - suffix.len();
        Ok((coordinates, len))
    }
}

fn check_non_local(string: &str) -> Result<(), String> {
    if string.starts_with('^') {
        Err(CANNOT_MIX.to_string())
    } else {
        Ok(())
    }
}

fn parse_relative(string: &str) -> (bool, usize) {
    if string.starts_with('~') {
        (true, '~'.len_utf8())
    } else {
        (false, 0)
    }
}

trait Number: brigadier::Number + Default + Into<f64> {}
impl Number for i32 {}
impl Number for f64 {}

fn parse_number_or_default<N: Number>(string: &str) -> Result<(N, usize), String> {
    match parse_number(string) {
        Ok(number) => Ok(number),
        Err(ParseNumberError::Empty(..)) => Ok((N::default(), 0)),
        Err(e) => Err(e.to_string()),
    }
}
