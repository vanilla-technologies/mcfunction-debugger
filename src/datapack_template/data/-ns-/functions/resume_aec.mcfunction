execute as @e[type=area_effect_cloud] run function -ns-:unfreeze_aec

scoreboard players set resume_success -ns-_global 0

# -resume_cases-

execute if score resume_success -ns-_global matches 0 run tellraw @a [{"text": "Breakpoint ","color": "red"},{"selector":"@s","color": "red"},{"text": " was deleted","color": "red"}]
execute unless score resume_success -ns-_global matches 0 run kill @s
