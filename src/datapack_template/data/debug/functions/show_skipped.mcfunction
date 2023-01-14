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

# -if_not_adapter-
execute if score skipped_missing -ns-_global matches 0 if score skipped_invalid -ns-_global matches 0 run tellraw @s [{"text":""},{"text":"[Info]","color":"blue","hoverEvent":{"action":"show_text","contents":"mcfunction-Debugger"}},{"text":" No functions were skipped."}]

execute if score skipped_missing -ns-_global matches 1.. run tellraw @s [{"text":""},{"text":"[Warning]","color":"gold","hoverEvent":{"action":"show_text","contents":"mcfunction-Debugger"}},{"text":" The following missing functions were skipped:"}]

# -missing_functions-

execute if score skipped_invalid -ns-_global matches 1.. run tellraw @s [{"text":""},{"text":"[Warning]","color":"gold","hoverEvent":{"action":"show_text","contents":"mcfunction-Debugger"}},{"text":" The following invalid functions were skipped:"}]

# -invalid_functions-
