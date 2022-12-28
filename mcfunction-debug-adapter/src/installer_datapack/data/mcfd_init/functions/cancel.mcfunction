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

execute at @e[type=area_effect_cloud,tag=mcfd_installer] run setblock ~ ~ ~ redstone_block
execute at @e[type=area_effect_cloud,tag=mcfd_installer] run setblock ~ ~1 ~ activator_rail
execute at @e[type=area_effect_cloud,tag=mcfd_installer] run summon command_block_minecart ~ ~1 ~ {Command: "function minect:enable_logging", Tags: ["mcfd_install_canceller"], TrackOutput: false}
execute at @e[type=area_effect_cloud,tag=mcfd_installer] run summon command_block_minecart ~ ~1 ~ {CustomName: '{"text":"mcfd_init"}', Command: "tag @s add mcfd_init_cancelled", Tags: ["mcfd_install_canceller"], TrackOutput: false}
execute at @e[type=area_effect_cloud,tag=mcfd_installer] run summon command_block_minecart ~ ~1 ~ {Command: "function minect:reset_logging", Tags: ["mcfd_install_canceller"], TrackOutput: false}
execute at @e[type=area_effect_cloud,tag=mcfd_installer] run summon command_block_minecart ~ ~1 ~ {Command: "function mcfd_init:cancel_cleanup", Tags: ["mcfd_install_canceller"], TrackOutput: false}
