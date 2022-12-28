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

execute unless entity @e[type=area_effect_cloud,tag=-ns-_breakpoint] run tellraw @a [{"text":""},{"text":"[Error]","color":"red","hoverEvent":{"action":"show_text","contents":"mcfunction-Debugger"}},{"text":" Could not find breakpoint entity!\n Start a new debugging session with: "},{"text":"/function debug:<your_namespace>/<your_function>","clickEvent":{"action":"suggest_command","value":"/function debug:"},"hoverEvent":{"action":"show_text","contents":"Click for suggestions"},"color":"aqua"}]
execute unless entity @e[type=area_effect_cloud,tag=-ns-_breakpoint] run function -ns-:abort_session
execute as @e[type=area_effect_cloud,tag=-ns-_breakpoint] run function -ns-:resume_self
