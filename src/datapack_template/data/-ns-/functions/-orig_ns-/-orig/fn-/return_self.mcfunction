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

scoreboard players operation current -ns-_anchor = @s -ns-_anchor
scoreboard players reset found_continue_function -ns-_global

tag @s remove -ns-_active

# -return_cases-

execute if score found_continue_function -ns-_global matches 1 run kill @s
# -minect_log_conditional-
execute unless score found_continue_function -ns-_global matches 1 run summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"error+function_call_deleted"}'}
execute unless score found_continue_function -ns-_global matches 1 run tellraw @a [{"text":""},{"text":"[Error]","color":"red","hoverEvent":{"action":"show_text","contents":"mcfunction-Debugger"}},{"text":" Function call at "},{"selector":"@s"},{"text":" was deleted!"}]
execute unless score found_continue_function -ns-_global matches 1 run function -ns-:abort_session
