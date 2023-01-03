# mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any
# Minecraft mods.
#
# © Copyright (C) 2021-2023 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
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

execute if score breakpoint -ns-_global matches 1 at @e[type=area_effect_cloud,tag=-ns-_breakpoint] run function -ns-:animate_context

execute if score tick_resume -ns-_global matches 1 run function -ns-:resume_immediately
scoreboard players reset tick_resume -ns-_global

execute as @e[type=area_effect_cloud,tag=-ns-_schedule] run function -ns-:schedule

schedule function -ns-:tick_start 1t

# This area_effect_cloud will die next tick when Minecraft increments it's age after running schedules and command blocks, even if someone calls decrement_age
summon area_effect_cloud ~ ~ ~ {Tags: [-ns-, -ns-_before_age_increment]}
