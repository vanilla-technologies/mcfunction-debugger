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

scoreboard players set breakpoint -ns-_global 1
summon area_effect_cloud ~ ~ ~ {Age: -2147483648, Duration: -1, WaitTime: -2147483648, Tags: [-ns-, -ns-_breakpoint, -ns-+-orig_ns-+-orig+fn-+-line_number-], CustomName: '{"text":"-orig_ns-:-orig/fn-:-line_number-"}'}
teleport @e[type=area_effect_cloud,tag=-ns-_breakpoint] ~ ~ ~ ~ ~
execute as @e[type=area_effect_cloud,tag=!-ns-_frozen] run function -ns-:freeze_aec

function -ns-:skipped_functions_warning
tellraw @a [{"text":""},{"text":"[Info]","color":"blue","hoverEvent":{"action":"show_text","contents":"mcfunction-Debugger"}},{"text":" Suspended at breakpoint -orig_ns-:-orig/fn-:-line_number-\n To resume run: "},{"text":"/function debug:resume","clickEvent":{"action":"run_command","value":"/function debug:resume"},"hoverEvent":{"action":"show_text","contents":"Click to execute"},"color":"aqua"},{"text": "\n To stop run: "},{"text":"/function debug:stop","clickEvent":{"action":"run_command","value":"/function debug:stop"},"hoverEvent":{"action":"show_text","contents":"Click to execute"},"color":"aqua"}]

scoreboard players reset * -ns-_scores
function -ns-:update_scores

# -minect_log-
tag @s add stopped_at_breakpoint.-orig_ns-+-orig+fn-+-line_number-
