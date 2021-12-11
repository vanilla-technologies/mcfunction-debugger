# mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any
# Minecraft mods.
#
# © Copyright (C) 2021 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
#
# This file is part of mcfunction-debugger.
#
# mcfunction-debugger is free software: you can redistribute it and/or modify it under the terms of
# the GNU General Public License as published by the Free Software Foundation, either version 3 of
# the License, or (at your option) any later version.
#
# mcfunction-debugger is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
# without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License along with mcfunction-debugger.
# If not, see <http://www.gnu.org/licenses/>.

scoreboard players set version -ns-_version 1
scoreboard objectives add -ns-_Age dummy
scoreboard objectives add -ns-_Duration dummy
scoreboard objectives add -ns-_WaitTime dummy
scoreboard objectives add -ns-_anchor dummy
scoreboard objectives add -ns-_depth dummy
scoreboard objectives add -ns-_global dummy
scoreboard objectives add -ns-_invalid dummy
scoreboard objectives add -ns-_skipped dummy
scoreboard objectives add -ns-_tmp dummy

scoreboard objectives add -ns-_constant dummy
scoreboard players set 1 -ns-_constant 1
scoreboard players set 88 -ns-_constant 88

function -ns-:id/install
