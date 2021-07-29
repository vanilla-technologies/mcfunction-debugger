execute run summon area_effect_cloud ~ ~ ~ {Age: --ticks-, Duration: -ticks-, WaitTime: --ticks-, Tags: [-ns-, -ns-_schedule, schedule_-schedule_ns-_-schedule_fn-]}
execute if entity @e[type=area_effect_cloud,tag=-ns-_before_age_increment] as @e[type=area_effect_cloud,tag=schedule_-schedule_ns-_-schedule_fn-,nbt={Age: --ticks-}] run function -ns-:decrement_age
