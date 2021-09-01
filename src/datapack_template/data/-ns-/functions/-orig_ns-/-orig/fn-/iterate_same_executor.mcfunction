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

execute as @e[type=area_effect_cloud,tag=-ns-_selected_entity_marker] if score @s -ns-_depth = current -ns-_depth run tag @s add -ns-_tmp
execute as @e[type=area_effect_cloud,tag=-ns-_tmp,limit=1] run tag @s add -ns-_current
tag @e[type=area_effect_cloud] remove -ns-_tmp

execute as @e[type=area_effect_cloud,tag=-ns-_current] if score @s -ns-_depth = current -ns-_depth run tag @s add -ns-_tmp

execute if score current -ns-_anchor matches 0 at @e[type=area_effect_cloud,tag=-ns-_tmp] anchored feet run function -ns-:-orig_ns-/-orig/fn-/-line_numbers-
execute if score current -ns-_anchor matches 1 at @e[type=area_effect_cloud,tag=-ns-_tmp] anchored eyes run function -ns-:-orig_ns-/-orig/fn-/-line_numbers-

execute unless score breakpoint -ns-_global matches 1 as @e[type=area_effect_cloud,tag=-ns-_current] if score @s -ns-_depth = current -ns-_depth run kill @s

execute unless score breakpoint -ns-_global matches 1 as @e[type=area_effect_cloud,tag=-ns-_selected_entity_marker] if score @s -ns-_depth = current -ns-_depth run tag @s add -ns-_tmp
execute unless score breakpoint -ns-_global matches 1 if entity @e[type=area_effect_cloud,tag=-ns-_tmp] run function -ns-:-orig_ns-/-orig/fn-/iterate_same_executor
