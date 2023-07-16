# McFunction-Debugger is a debugger for Minecraft's *.mcfunction files that does not require any
# Minecraft mods.
#
# © Copyright (C) 2021-2023 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
#
# This file is part of McFunction-Debugger.
#
# McFunction-Debugger is free software: you can redistribute it and/or modify it under the terms of
# the GNU General Public License as published by the Free Software Foundation, either version 3 of
# the License, or (at your option) any later version.
#
# McFunction-Debugger is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
# without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License along with McFunction-Debugger.
# If not, see <http://www.gnu.org/licenses/>.

scoreboard players reset found_context -ns-_global
execute as @e[type=area_effect_cloud,tag=-ns-_context,tag=-ns-_active,tag=-ns-_current] if score @s -ns-_depth = current -ns-_depth run scoreboard players set found_context -ns-_global 1

# -minect_log_conditional-
execute unless score found_context -ns-_global matches 1 run summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"error+context_entity_killed"}'}
execute unless score found_context -ns-_global matches 1 run tellraw @a [{"text":""},{"text":"[Error]","color":"red","hoverEvent":{"action":"show_text","contents":"mcfunction-Debugger"}},{"text":" Debugger context entity was killed!"}]
execute unless score found_context -ns-_global matches 1 run function -ns-:abort_session
execute as @e[type=area_effect_cloud,tag=-ns-_context,tag=-ns-_active,tag=-ns-_current] if score @s -ns-_depth = current -ns-_depth run function -ns-:-orig_ns-/-orig/fn-/continue_at_-position-

scoreboard players set found_continue_function -ns-_global 1
