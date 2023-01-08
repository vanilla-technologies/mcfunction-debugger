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

pub mod adapter;

use crate::{
    config::adapter::{AdapterConfig, BreakpointKind, BreakpointPositionInLine},
    parser::command::resource_location::ResourceLocation,
};

pub struct Config<'l> {
    pub namespace: &'l str,
    pub shadow: bool,
    pub adapter: Option<AdapterConfig<'l>>,
}
impl Config<'_> {
    pub(crate) fn get_breakpoint_kind(
        &self,
        function: &ResourceLocation,
        line_number: usize,
        position_in_line: BreakpointPositionInLine,
    ) -> Option<&BreakpointKind> {
        if let Some(config) = self.adapter.as_ref() {
            if let Some(vec) = config.breakpoints.get_vec(function) {
                return vec
                    .iter()
                    .filter(|breakpoint| breakpoint.position.line_number == line_number)
                    .filter(|breakpoint| breakpoint.position.position_in_line == position_in_line)
                    .next()
                    .map(|it| &it.kind);
            }
        }
        None
    }
}
