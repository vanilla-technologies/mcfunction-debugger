execute as @e[type=area_effect_cloud,tag=-ns-_max_age] run function -ns-:eval_max_age_tag

scoreboard players set resume_success -ns-_global 0

# -resume_cases-

execute if score resume_success -ns-_global matches 0 run tellraw @a [{"text": "Breakpoint ","color": "red"},{"selector":"@s","color": "red"},{"text": " was deleted","color": "red"}]
execute unless score resume_success -ns-_global matches 0 run kill @s
