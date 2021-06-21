execute unless score breakpoint namespace_breakpoint matches 1 run tellraw @a {"text": "Not currently standing at a breakpoint","color": "red"}
execute if score breakpoint namespace_breakpoint matches 1 unless entity @e[type=area_effect_cloud,tag=namespace_breakpoint] run tellraw @a {"text": "Could not find breakpoint entity","color": "red"}
execute as @e[type=area_effect_cloud,tag=namespace_breakpoint] run function namespace:continue_aec
