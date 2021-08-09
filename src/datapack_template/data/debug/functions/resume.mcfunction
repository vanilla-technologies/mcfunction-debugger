execute unless score breakpoint -ns-_global matches 1 run tellraw @a {"text": "Cannot resume, no function is suspended at a breakpoint!","color": "red"}
scoreboard players operation resume_success -ns-_global = breakpoint -ns-_global
scoreboard players set breakpoint -ns-_global 0

execute if score resume_success -ns-_global matches 1 if entity @e[type=area_effect_cloud,tag=-ns-_before_age_increment,tag=!-ns-_frozen] if entity @e[type=area_effect_cloud,tag=-ns-_before_age_increment,tag=-ns-_frozen] run function -ns-:resume_immediate
execute if score resume_success -ns-_global matches 1 unless entity @e[type=area_effect_cloud,tag=-ns-_before_age_increment,tag=!-ns-_frozen] unless entity @e[type=area_effect_cloud,tag=-ns-_before_age_increment,tag=-ns-_frozen] run function -ns-:resume_immediate
execute if score resume_success -ns-_global matches 1 if entity @e[type=area_effect_cloud,tag=-ns-_before_age_increment,tag=!-ns-_frozen] unless entity @e[type=area_effect_cloud,tag=-ns-_before_age_increment,tag=-ns-_frozen] run scoreboard players set tick_resume -ns-_global 1
execute if score resume_success -ns-_global matches 1 unless entity @e[type=area_effect_cloud,tag=-ns-_before_age_increment,tag=!-ns-_frozen] if entity @e[type=area_effect_cloud,tag=-ns-_before_age_increment,tag=-ns-_frozen] run schedule function -ns-:resume_immediate 1t
