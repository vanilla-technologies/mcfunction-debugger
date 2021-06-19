# This area_effect_cloud will die next tick when Minecraft increments it's age after running schedules and command blocks, even if someone calls decrement_age
summon area_effect_cloud ~ ~ ~ {Tags: [namespace_before_age_increment]}

execute as @e[type=area_effect_cloud,tag=namespace_schedule,nbt={Age: -1}] run function debug:schedule

schedule function debug:tick_start 1t
