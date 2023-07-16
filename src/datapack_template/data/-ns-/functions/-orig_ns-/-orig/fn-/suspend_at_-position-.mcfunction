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

summon area_effect_cloud ~ ~ ~ {Age: -2147483648, Duration: -1, WaitTime: -2147483648, Tags: [-ns-, -ns-_breakpoint, -ns-+-orig_ns-+-orig+fn-+-position-], CustomName: '{"text":"-orig_ns-:-orig/fn-:-line_number--optional_column-"}'}
teleport @e[type=area_effect_cloud,tag=-ns-_breakpoint] ~ ~ ~ ~ ~
execute as @e[type=area_effect_cloud,tag=!-ns-_frozen] run function -ns-:freeze_aec

function -ns-:skipped_functions_warning

# -minect_log-
summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"stopped+-orig_ns-+-orig+fn-+-position-"}'}
