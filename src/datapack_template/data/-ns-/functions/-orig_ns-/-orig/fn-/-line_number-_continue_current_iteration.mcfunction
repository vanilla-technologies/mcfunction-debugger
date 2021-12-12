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

scoreboard players reset found_context -ns-_global
execute as @e[type=area_effect_cloud,tag=-ns-_context,tag=-ns-_active,tag=-ns-_current] if score @s -ns-_depth = current -ns-_depth run scoreboard players set found_context -ns-_global 1
execute unless score found_context -ns-_global matches 1 run tellraw @a [{"text":""},{"text":"[Error]","color":"red","hoverEvent":{"action":"show_text","contents":"mcfunction-Debugger"}},{"text":" Debugger context entity was killed!\n Start a new debugging session with '/function debug:<your_namespace>/<your_function>'"}]
execute unless score found_context -ns-_global matches 1 run function -ns-:clean_up
execute as @e[type=area_effect_cloud,tag=-ns-_context,tag=-ns-_active,tag=-ns-_current] if score @s -ns-_depth = current -ns-_depth run function -ns-:-orig_ns-/-orig/fn-/-line_number-_continue

scoreboard players set found_continue_function -ns-_global 1
