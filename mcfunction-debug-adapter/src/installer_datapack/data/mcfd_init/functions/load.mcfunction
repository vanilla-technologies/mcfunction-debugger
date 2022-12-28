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

function mcfd_init:install

scoreboard players set connection_exists mcfd_init_global 0
execute at @e[type=area_effect_cloud,tag=minect_connection] positioned ~ ~-3 ~ if block ~ ~ ~ structure_block run scoreboard players set connection_exists mcfd_init_global 1

execute unless score connection_exists mcfd_init_global matches 1 run tellraw @a [{"text":""},{"text":"[Info]","color":"blue","hoverEvent":{"action":"show_text","contents":"mcfunction-Debugger"}},{"text":" Establishing connection to debugger.\n You can click on the colored text below to choose a chunk in which to generate the connection structure. The chunk may be cleared by the connection, so make sure it does not contain anything important.\n "},{"text":"[Choose a chunk]","clickEvent":{"action":"suggest_command","value":"/execute positioned ~ ~ ~ run function mcfd_init:choose_chunk"},"hoverEvent":{"action":"show_text","contents":"Click for suggestions"},"color":"aqua"}]
