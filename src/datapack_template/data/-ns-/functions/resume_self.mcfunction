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

tellraw @a [{"text":"[Info]","color":"blue"},{"text":" Resuming debugging from ","color":"white"},{"selector":"@s","color":"white"}]

scoreboard players reset found_continue_function -ns-_global

# -resume_cases-

execute if score found_continue_function -ns-_global matches 1 run kill @s
execute unless score found_continue_function -ns-_global matches 1 run tellraw @a [{"text":"[Error]","color":"red"},{"text":" Breakpoint at ","color":"white"},{"selector":"@s","color":"white"},{"text":" was deleted!\n You can either restore this breakpoint or stop the current debugging session with ","color":"white"},{"text":"/function debug:stop","clickEvent":{"action":"suggest_command","value":"/function debug:stop"},"color":"aqua"}]
execute unless score found_continue_function -ns-_global matches 1 run scoreboard players set breakpoint -ns-_global 1
