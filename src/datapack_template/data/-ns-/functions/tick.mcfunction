# This area_effect_cloud will die next tick when Minecraft increments it's age after running schedules and command blocks, even if someone calls decrement_age
summon area_effect_cloud ~ ~ ~ {Tags: [-ns-, -ns-_before_age_increment]}

execute as @e[type=area_effect_cloud,tag=-ns-_schedule] run function -ns-:schedule

schedule function -ns-:tick_start 1t
