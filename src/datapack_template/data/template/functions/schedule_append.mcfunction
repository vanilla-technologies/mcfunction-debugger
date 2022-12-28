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

execute run summon area_effect_cloud ~ ~ ~ {Age: --ticks-, Duration: -ticks-, WaitTime: --ticks-, Tags: [-ns-, -ns-_schedule, -ns-+schedule+-schedule_ns-+-schedule+fn-]}
execute if entity @e[type=area_effect_cloud,tag=-ns-_before_age_increment] as @e[type=area_effect_cloud,tag=-ns-+schedule+-schedule_ns-+-schedule+fn-,nbt={Age: --ticks-},limit=1] run function -ns-:decrement_age
