execute unless entity @e[type=area_effect_cloud,tag=-ns-_breakpoint] run tellraw @a {"text": "Could not find breakpoint entity!","color": "red"}
execute as @e[type=area_effect_cloud,tag=-ns-_breakpoint] run function -ns-:resume_self
