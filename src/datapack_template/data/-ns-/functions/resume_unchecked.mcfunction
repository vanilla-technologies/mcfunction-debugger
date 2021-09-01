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

scoreboard players set breakpoint -ns-_global 0

scoreboard players set resume_time_within_tick -ns-_global 0
execute if entity @e[type=area_effect_cloud,tag=-ns-_before_age_increment,tag=!-ns-_frozen] run scoreboard players add resume_time_within_tick -ns-_global 1
execute if entity @e[type=area_effect_cloud,tag=-ns-_before_age_increment,tag=-ns-_frozen] run scoreboard players remove resume_time_within_tick -ns-_global 1

# We are at the correct time within a tick -> resume immediately
execute if score resume_time_within_tick -ns-_global matches 0 run function -ns-:resume_immediately
# We are after age increment, but need to resume before age increment -> resume in schedule
execute if score resume_time_within_tick -ns-_global matches -1 run schedule function -ns-:resume_immediately 1t
# We are before age increment, but need to resume after age increment -> resume in tick.json
execute if score resume_time_within_tick -ns-_global matches 1 run scoreboard players set tick_resume -ns-_global 1
