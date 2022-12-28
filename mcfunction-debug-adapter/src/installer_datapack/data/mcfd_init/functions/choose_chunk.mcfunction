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

function mcfd_init:remove_chunk_choice
summon area_effect_cloud ~ ~ ~ {Duration: 2147483647, Tags: ["mcfd", "mcfd_installer"]}

execute store result score entityX mcfd_init_global run data get entity @e[type=area_effect_cloud,tag=mcfd_installer,limit=1] Pos[0] 1
scoreboard players operation chunkX mcfd_init_global = entityX mcfd_init_global
scoreboard players operation entityX mcfd_init_global %= 16 mcfd_init_const
scoreboard players operation chunkX mcfd_init_global -= entityX mcfd_init_global
execute store result entity @e[type=area_effect_cloud,tag=mcfd_installer,limit=1] Pos[0] double 1 run scoreboard players get chunkX mcfd_init_global

execute store result score entityZ mcfd_init_global run data get entity @e[type=area_effect_cloud,tag=mcfd_installer,limit=1] Pos[2] 1
scoreboard players operation chunkZ mcfd_init_global = entityZ mcfd_init_global
scoreboard players operation entityZ mcfd_init_global %= 16 mcfd_init_const
scoreboard players operation chunkZ mcfd_init_global -= entityZ mcfd_init_global
execute store result entity @e[type=area_effect_cloud,tag=mcfd_installer,limit=1] Pos[2] double 1 run scoreboard players get chunkZ mcfd_init_global

execute at @e[type=area_effect_cloud,tag=mcfd_installer,limit=1] run setblock ~ ~ ~ structure_block{name: "minect:dap/-structure_id-", mode: "LOAD", showboundingbox: true, sizeX: 16, sizeY: 256, sizeZ: 16}

tellraw @a [{"text":""},{"text":"[Info]","color":"blue","hoverEvent":{"action":"show_text","contents":"mcfunction-Debugger"}},{"text":" This chunk will be force loaded to keep the connection active when no player is around.\n "},{"text":"[Confirm]","clickEvent":{"action":"run_command","value":"/function mcfd_init:confirm_chunk"},"hoverEvent":{"action":"show_text","contents":"Click to execute"},"color":"aqua"},{"text": " "},{"text":"[Choose different chunk]","clickEvent":{"action":"suggest_command","value":"/execute positioned ~ ~ ~ run function mcfd_init:choose_chunk"},"hoverEvent":{"action":"show_text","contents":"Click for suggestions"},"color":"aqua"},{"text": " "},{"text":"[Cancel]","clickEvent":{"action":"run_command","value":"/function mcfd_init:cancel"},"hoverEvent":{"action":"show_text","contents":"Click to execute"},"color":"aqua"}]
