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

execute unless score @s -ns-_id matches 0 run scoreboard players operation @e[tag=!-ns-_context] -ns-_id -= @s -ns-_id
execute unless score @s -ns-_id matches 0 as @e[tag=!-ns-_context,scores={-ns-_id=0}] run tag @s add -ns-_tmp
execute unless score @s -ns-_id matches 0 run scoreboard players operation @e[tag=!-ns-_context] -ns-_id += @s -ns-_id

execute unless score @s -ns-_id matches 0 unless entity @e[tag=!-ns-_context,tag=-ns-_tmp] run tellraw @a [{"text":""},{"text":"[Error]","color":"red","hoverEvent":{"action":"show_text","contents":"mcfunction-Debugger"}},{"text":" Selected entity was killed!\n Start a new debugging session with: "},{"text":"/function debug:<your_namespace>/<your_function>","clickEvent":{"action":"suggest_command","value":"/function debug:"},"hoverEvent":{"action":"show_text","contents":"Click for suggestions"},"color":"aqua"}]
execute unless score @s -ns-_id matches 0 unless entity @e[tag=!-ns-_context,tag=-ns-_tmp] run function -ns-:abort_session
execute if score @s -ns-_id matches 0 at @s run function -ns-:-orig_ns-/-orig/fn-/-line_numbers-
execute if score current -ns-_anchor matches 0 at @s as @e[tag=!-ns-_context,tag=-ns-_tmp] anchored feet run function -ns-:-orig_ns-/-orig/fn-/-line_numbers-
execute if score current -ns-_anchor matches 1 at @s as @e[tag=!-ns-_context,tag=-ns-_tmp] anchored eyes run function -ns-:-orig_ns-/-orig/fn-/-line_numbers-
