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

execute if score -call_ns-:-call/fn- -ns-_valid matches 1 run scoreboard players add current -ns-_depth 1

execute if score -call_ns-:-call/fn- -ns-_valid matches 1 run summon area_effect_cloud ~ ~ ~ {Duration: 2147483647, Tags: [-ns-_new, -ns-, -ns-_function_call, -ns-_-orig_ns-_-orig_fn-_-line_number-, -ns-_active], CustomName: '{"text":"-orig_ns-:-orig/fn-:-line_number-"}'}
execute if score -call_ns-:-call/fn- -ns-_valid matches 1 run scoreboard players operation @e[type=area_effect_cloud,tag=-ns-_new] -ns-_anchor = current -ns-_anchor
execute if score -call_ns-:-call/fn- -ns-_valid matches 1 run scoreboard players operation @e[type=area_effect_cloud,tag=-ns-_new] -ns-_depth = current -ns-_depth
execute if score -call_ns-:-call/fn- -ns-_valid matches 1 run tag @e[type=area_effect_cloud,tag=-ns-_new] remove -ns-_new

# -debug_anchor-

execute if score -call_ns-:-call/fn- -ns-_valid matches 1 run execute run function -ns-:select_entity
execute if score -call_ns-:-call/fn- -ns-_valid matches 1 run function -ns-:-call_ns-/-call/fn-/next_iteration_or_return

execute unless score -call_ns-:-call/fn- -ns-_valid matches 1 run scoreboard players add skipped_calls -ns-_global 1
execute unless score -call_ns-:-call/fn- -ns-_valid matches 0.. unless score -call_ns-:-call/fn- -ns-_skipped matches 1.. run scoreboard players add skipped_missing -ns-_global 1
execute if score -call_ns-:-call/fn- -ns-_valid matches 0 unless score -call_ns-:-call/fn- -ns-_skipped matches 1.. run scoreboard players add skipped_invalid -ns-_global 1
execute unless score -call_ns-:-call/fn- -ns-_valid matches 1 run scoreboard players add -call_ns-:-call/fn- -ns-_skipped 1
execute unless score -call_ns-:-call/fn- -ns-_valid matches 1 run function -ns-:-orig_ns-/-orig/fn-/-line_number_1-_continue_current_iteration
