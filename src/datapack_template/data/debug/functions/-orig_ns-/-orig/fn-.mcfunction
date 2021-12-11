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

execute if score breakpoint -ns-_global matches 1 run tellraw @s [{"text":"[Error]","color":"red"},{"text":" Cannot start debugging -orig_ns-:-orig/fn-, because a function is already suspended at a breakpoint!\n To resume run: ","color":"white"},{"text":"/function debug:resume","clickEvent":{"action":"run_command","value":"/function debug:resume"},"color":"aqua"}]
execute unless score breakpoint -ns-_global matches 1 run function -ns-:-orig_ns-/-orig/fn-/start
