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

execute if score skipped_calls -ns-_global matches 1.. run tellraw @a [{"text":""},{"text":"[Warning]","color":"gold","hoverEvent":{"action":"show_text","contents":"mcfunction-Debugger"}},{"text":" "},{"score":{"name":"skipped_missing","objective":"-ns-_global"}},{"text":" missing and "},{"score":{"name":"skipped_invalid","objective":"-ns-_global"}},{"text":" invalid functions were skipped.\n For more details run: "},{"text":"/function debug:show_skipped","clickEvent":{"action":"run_command","value":"/function debug:show_skipped"},"hoverEvent":{"action":"show_text","contents":"Click to execute"},"color":"aqua"}]
