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

scoreboard players add current -ns-_depth 1

summon area_effect_cloud ~ ~ ~ {Duration: 2147483647, Tags: [-ns-_new, -ns-, -ns-_function_call, -ns-_-orig_ns-_-orig_fn-, -ns-_active]}
scoreboard players operation @e[type=area_effect_cloud,tag=-ns-_new] -ns-_anchor = current -ns-_anchor
scoreboard players operation @e[type=area_effect_cloud,tag=-ns-_new] -ns-_depth = current -ns-_depth
tag @e[type=area_effect_cloud,tag=-ns-_new] remove -ns-_new

# -debug_anchor-

execute run function -ns-:select_entity
function -ns-:-call_ns-/-call/fn-/next_iteration_or_return
