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

use crate::{
    config::{
        adapter::{BreakpointKind, BreakpointPositionInLine},
        Config,
    },
    parser::{
        command::{argument::MinecraftEntityAnchor, resource_location::ResourceLocation},
        Line,
    },
};
use std::{collections::BTreeSet, fmt::Display, str::FromStr};

pub(crate) struct Partition<'l> {
    pub(crate) start: Position,
    pub(crate) end: Position,
    pub(crate) regular_lines: &'l [(usize, String, Line)],
    pub(crate) terminator: Terminator<'l>,
}

pub(crate) enum Terminator<'l> {
    Breakpoint,
    Step {
        condition: &'l str,
        position_in_line: BreakpointPositionInLine,
    },
    Continue {
        position_in_line: BreakpointPositionInLine,
    },
    FunctionCall {
        line: &'l str,
        name: &'l ResourceLocation,
        anchor: &'l Option<MinecraftEntityAnchor>,
        selectors: &'l BTreeSet<usize>,
    },
    Return,
}
impl Terminator<'_> {
    fn get_position_in_line(&self) -> PositionInLine {
        match self {
            Terminator::Breakpoint => PositionInLine::Breakpoint,
            Terminator::Step {
                position_in_line, ..
            } => (*position_in_line).into(),
            Terminator::Continue { position_in_line } => (*position_in_line).into(),
            Terminator::FunctionCall { .. } => PositionInLine::Function,
            Terminator::Return => PositionInLine::Return,
        }
    }
}

pub(crate) fn partition<'l>(
    function: &ResourceLocation,
    lines: &'l [(usize, String, Line)],
    config: &'l Config,
) -> Vec<Partition<'l>> {
    let mut partitions = Vec::new();
    let mut start_line_index = 0;
    let mut start = Position {
        line_number: 0,
        position_in_line: PositionInLine::Entry,
    };
    // TODO: Can we remove line_number from the triple?
    for (line_index, (_line_number, line, command)) in lines.iter().enumerate() {
        let line_number = line_index + 1;
        let mut next_partition = |terminator: Terminator<'l>| {
            let end = Position {
                line_number,
                position_in_line: terminator.get_position_in_line(),
            };
            let partition = Partition {
                start,
                end,
                regular_lines: &lines[start_line_index..line_index],
                terminator,
            };
            start = end;
            start_line_index = line_index;
            partition
        };
        let get_breakpoint_terminator = |position_in_line| match config.get_breakpoint_kind(
            function,
            line_number,
            position_in_line,
        ) {
            Some(BreakpointKind::Normal) => Some(Terminator::Breakpoint),
            Some(BreakpointKind::Invalid) => None,
            Some(BreakpointKind::Continue) => Some(Terminator::Continue { position_in_line }),
            Some(BreakpointKind::Step { condition }) => Some(Terminator::Step {
                condition,
                position_in_line,
            }),
            None => None,
        };

        if let Some(terminator) = get_breakpoint_terminator(BreakpointPositionInLine::Breakpoint) {
            partitions.push(next_partition(terminator));
        }
        if matches!(command, Line::Breakpoint) {
            partitions.push(next_partition(Terminator::Breakpoint));
        }
        if let Line::FunctionCall {
            name,
            anchor,
            selectors,
            ..
        } = command
        {
            partitions.push(next_partition(Terminator::FunctionCall {
                line,
                name,
                anchor,
                selectors,
            }));
        }
        if let Some(terminator) = get_breakpoint_terminator(BreakpointPositionInLine::AfterFunction)
        {
            partitions.push(next_partition(terminator));
        }

        if matches!(command, Line::Breakpoint | Line::FunctionCall { .. }) {
            start_line_index += 1; // Skip the line containing the breakpoint / function call
        }
    }
    partitions.push(Partition {
        start,
        end: Position {
            line_number: lines.len(),
            position_in_line: PositionInLine::Return,
        },
        regular_lines: &lines[start_line_index..lines.len()],
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
