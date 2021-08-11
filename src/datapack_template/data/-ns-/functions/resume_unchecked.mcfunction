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
