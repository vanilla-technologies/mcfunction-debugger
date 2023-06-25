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

use crate::partition::PositionInLine;
use std::{fmt::Display, str::FromStr};

pub struct AdapterConfig<'l> {
    pub adapter_listener_name: &'l str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalBreakpointPosition {
    pub line_number: usize,
    pub position_in_line: BreakpointPositionInLine,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BreakpointPositionInLine {
    Breakpoint,
    AfterFunction,
}
impl From<BreakpointPositionInLine> for PositionInLine {
    fn from(value: BreakpointPositionInLine) -> Self {
        match value {
            BreakpointPositionInLine::Breakpoint => PositionInLine::Breakpoint,
            BreakpointPositionInLine::AfterFunction => PositionInLine::AfterFunction,
        }
    }
}
impl FromStr for BreakpointPositionInLine {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "breakpoint" => Ok(BreakpointPositionInLine::Breakpoint),
            "after_function" => Ok(BreakpointPositionInLine::AfterFunction),
            _ => Err(()),
        }
    }
}
impl Display for BreakpointPositionInLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BreakpointPositionInLine::Breakpoint => write!(f, "breakpoint"),
            BreakpointPositionInLine::AfterFunction => write!(f, "after_function"),
        }
    }
}
