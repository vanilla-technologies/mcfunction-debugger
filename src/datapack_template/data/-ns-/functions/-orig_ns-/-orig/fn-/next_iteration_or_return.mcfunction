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

# Skip execution if function contains invalid commands
execute if score -orig_ns-:-orig/fn- -ns-_invalid matches 1 unless score -orig_ns-:-orig/fn- -ns-_skipped matches 1.. run scoreboard players add skipped -ns-_skipped 1
execute if score -orig_ns-:-orig/fn- -ns-_invalid matches 1 run scoreboard players add -orig_ns-:-orig/fn- -ns-_skipped 1
execute if score -orig_ns-:-orig/fn- -ns-_invalid matches 1 as @e[type=area_effect_cloud,tag=-ns-_context,tag=-ns-_active] if score @s -ns-_depth = current -ns-_depth run kill @s

# Select next context
execute as @e[type=area_effect_cloud,tag=-ns-_context,tag=-ns-_active] if score @s -ns-_depth = current -ns-_depth run tag @s add -ns-_tmp
execute as @e[type=area_effect_cloud,tag=-ns-_tmp,limit=1] run tag @s add -ns-_current

# If there is no entity with -ns-_tmp, we return.
execute unless entity @e[type=area_effect_cloud,tag=-ns-_tmp] run function -ns-:-orig_ns-/-orig/fn-/return_or_finish

tag @e[type=area_effect_cloud] remove -ns-_tmp

# If we returned above, the program is now either
# 1. suspended at a breakpoint or
# 2. terminated, in which case there is no entity with tag=-ns-_current
execute unless score breakpoint -ns-_global matches 1 as @e[type=area_effect_cloud,tag=-ns-_context,tag=-ns-_active,tag=-ns-_current] if score @s -ns-_depth = current -ns-_depth run function -ns-:-orig_ns-/-orig/fn-/-line_number-_continue
