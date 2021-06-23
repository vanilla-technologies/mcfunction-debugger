execute unless score breakpoint -ns-_global matches 1 run tellraw @a {"text": "Not currently standing at a breakpoint","color": "red"}
execute if score breakpoint -ns-_global matches 1 unless entity @e[type=area_effect_cloud,tag=-ns-_breakpoint] run tellraw @a {"text": "Could not find breakpoint entity","color": "red"}
execute as @e[type=area_effect_cloud,tag=-ns-_breakpoint] run function -ns-:continue_aec
