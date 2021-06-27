scoreboard players set schedule_success -ns-_global 0
execute run scoreboard players set schedule_success -ns-_global 1
execute if score schedule_success -ns-_global matches 1 run kill @e[type=area_effect_cloud,tag=schedule_-schedule_ns-_-schedule_fn-]
execute if score schedule_success -ns-_global matches 1 run summon area_effect_cloud ~ ~ ~ {Age: --ticks-, Duration: -ticks-, WaitTime: --ticks-, Tags: [-ns-_schedule, schedule_-schedule_ns-_-schedule_fn-]}
