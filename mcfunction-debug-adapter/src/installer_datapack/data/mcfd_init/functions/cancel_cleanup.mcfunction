# mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any
# Minecraft mods.
#
# © Copyright (C) 2021, 2022 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
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

kill @e[type=command_block_minecart,tag=mcfd_install_canceller]
function mcfd_init:remove_chunk_choice
function mcfd_init:uninstall

tellraw @a [{"text":""},{"text":"[Info]","color":"blue","hoverEvent":{"action":"show_text","contents":"mcfunction-Debugger"}},{"text":" Installation cancelled."}]

# This loads the removal of the installer datapack on disk
schedule function mcfd_init:reload 1t