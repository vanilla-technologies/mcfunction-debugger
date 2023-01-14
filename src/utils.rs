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

pub trait Map0<T0, R0> {
    type Output;

    fn map0<F: Fn(T0) -> R0>(self, map: F) -> Self::Output;
}

impl<T0, T1, R0> Map0<T0, R0> for (T0, T1) {
    type Output = (R0, T1);

    fn map0<F: Fn(T0) -> R0>(self, map: F) -> Self::Output {
        (map(self.0), self.1)
    }
}
