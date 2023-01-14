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

# Select next context
execute as @e[type=area_effect_cloud,tag=-ns-_context,tag=-ns-_active] if score @s -ns-_depth = current -ns-_depth run tag @s add -ns-_tmp
execute as @e[type=area_effect_cloud,tag=-ns-_tmp,limit=1] run tag @s add -ns-_current

# If there is no entity with -ns-_tmp, we return.
execute unless entity @e[type=area_effect_cloud,tag=-ns-_tmp] run function -ns-:-orig_ns-/-orig/fn-/return_or_exit

tag @e[type=area_effect_cloud] remove -ns-_tmp

# If we returned above, the program is now either
# 1. suspended at a breakpoint or
# 2. terminated, in which case there is no entity with tag=-ns-_current
execute unless score breakpoint -ns-_global matches 1 as @e[type=area_effect_cloud,tag=-ns-_context,tag=-ns-_active,tag=-ns-_current] if score @s -ns-_depth = current -ns-_depth run function -ns-:-orig_ns-/-orig/fn-/continue_at_-position-
