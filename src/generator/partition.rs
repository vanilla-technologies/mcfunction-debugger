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

use crate::generator::parser::{
    command::{argument::MinecraftEntityAnchor, resource_location::ResourceLocation},
    Line,
};
use std::{collections::BTreeSet, fmt::Display, str::FromStr};

pub(crate) struct Partition<'l> {
    pub(crate) start: Position,
    pub(crate) end: Position,
    pub(crate) regular_lines: &'l [(usize, String, Line)],
    pub(crate) terminator: Terminator<'l>,
}

pub(crate) enum Terminator<'l> {
    ConfigurableBreakpoint {
        position_in_line: BreakpointPositionInLine,
    },
    FunctionCall {
        column_index: usize,
        line: &'l str,
        name: &'l ResourceLocation,
        anchor: &'l Option<MinecraftEntityAnchor>,
        selectors: &'l BTreeSet<usize>,
    },
    Return,
}

pub(crate) fn partition<'l>(lines: &'l [(usize, String, Line)]) -> Vec<Partition<'l>> {
    let mut partitions = Vec::new();
    let mut end = Position {
        line_number: 1,
        position_in_line: PositionInLine::Entry,
    };

    // TODO: Can we remove line_number from the triple?
    for (line_index, (_line_number, line, command)) in lines.iter().enumerate() {
        let line_number = line_index + 1;
        if let Line::Empty | Line::Comment = command {
            continue;
        }
        let start = end;
        end = Position {
            line_number,
            position_in_line: PositionInLine::Breakpoint,
        };
        let include_start_line = start.position_in_line == PositionInLine::Entry
            || start.position_in_line == PositionInLine::Breakpoint;
        let start_regular_line_index = start.line_number - if include_start_line { 1 } else { 0 };
        partitions.push(Partition {
            start,
            end,
            regular_lines: &lines[start_regular_line_index..line_index],
            terminator: Terminator::ConfigurableBreakpoint {
                position_in_line: BreakpointPositionInLine::Breakpoint,
            },
        });

        if let Line::FunctionCall {
            column_index,
            name,
            anchor,
            selectors,
            ..
        } = command
        {
            let start = end;
            end = Position {
                line_number,
                position_in_line: PositionInLine::Function,
            };
            partitions.push(Partition {
                start,
                end,
                regular_lines: &[],
                terminator: Terminator::FunctionCall {
                    column_index: *column_index,
                    line,
                    name,
                    anchor,
                    selectors,
                },
            });
        }
    }
    if end.position_in_line == PositionInLine::Entry {
        let start = end;
        end = Position {
            line_number: start.line_number,
            position_in_line: PositionInLine::Breakpoint,
        };
        partitions.push(Partition {
            start,
            end,
            regular_lines: &[],
            terminator: Terminator::ConfigurableBreakpoint {
                position_in_line: BreakpointPositionInLine::Breakpoint,
            },
        })
    }

    if end.position_in_line == PositionInLine::Function {
        let start = end;
        let last_line = start.line_number >= lines.len();
        let line_number = start.line_number + if last_line { 0 } else { 1 };
        let position_in_line = if last_line {
            BreakpointPositionInLine::AfterFunction
        } else {
            BreakpointPositionInLine::Breakpoint
        };
        end = Position {
            line_number,
            position_in_line: position_in_line.into(),
        };
        partitions.push(Partition {
            start,
            end,
            regular_lines: &[],
            terminator: Terminator::ConfigurableBreakpoint { position_in_line },
        });
    }

    let start = end;
    end = Position {
        line_number: lines.len(),
        position_in_line: PositionInLine::Return,
    };
    let start_regular_line_index = start.line_number
        - if start.position_in_line == PositionInLine::Breakpoint {
            1
        } else {
            0
        };
    partitions.push(Partition {
        start,
        end,
        regular_lines: &lines[start_regular_line_index..lines.len()],
        terminator: Terminator::Return,
    });
    partitions
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Position {
    pub(crate) line_number: usize,
    pub(crate) position_in_line: PositionInLine,
}
impl FromStr for Position {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn from_str_inner(s: &str) -> Option<Position> {
            let (line_number, position_in_line) = s.split_once('_')?;
            let line_number = line_number.parse().ok()?;
            let position_in_line = position_in_line.parse().ok()?;
            Some(Position {
                line_number,
                position_in_line,
            })
        }
        from_str_inner(s).ok_or(())
    }
}
impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.line_number, self.position_in_line)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PositionInLine {
    Entry,
    Breakpoint,
    Function,
    AfterFunction,
    Return,
}
impl FromStr for PositionInLine {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "entry" => Ok(PositionInLine::Entry),
            "breakpoint" => Ok(PositionInLine::Breakpoint),
            "function" => Ok(PositionInLine::Function),
            "after_function" => Ok(PositionInLine::AfterFunction),
            "return" => Ok(PositionInLine::Return),
            _ => Err(()),
        }
    }
}
impl Display for PositionInLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionInLine::Entry => write!(f, "entry"),
            PositionInLine::Breakpoint => write!(f, "breakpoint"),
            PositionInLine::Function => write!(f, "function"),
            PositionInLine::AfterFunction => write!(f, "after_function"),
            PositionInLine::Return => write!(f, "return"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalBreakpointPosition {
    pub line_number: usize,
    pub position_in_line: BreakpointPositionInLine,
}
impl FromStr for LocalBreakpointPosition {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn from_str_inner(s: &str) -> Option<LocalBreakpointPosition> {
            let (line_number, position_in_line) = s.split_once('_')?;
            let line_number = line_number.parse().ok()?;
            let position_in_line = position_in_line.parse().ok()?;
            Some(LocalBreakpointPosition {
                line_number,
                position_in_line,
            })
        }
        from_str_inner(s).ok_or(())
    }
}
impl Display for LocalBreakpointPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.line_number, self.position_in_line)
    }
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
