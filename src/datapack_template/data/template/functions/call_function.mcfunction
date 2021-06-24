scoreboard players add current -ns-_depth 1

summon area_effect_cloud ~ ~ ~ {Duration: 2147483647, Tags: [-ns-_new, -ns-, -ns-_function_call, -ns-_-orig_ns-_-orig_fn-]}
scoreboard players operation @e[type=area_effect_cloud,tag=-ns-_new] -ns-_anchor = current -ns-_anchor
scoreboard players operation @e[type=area_effect_cloud,tag=-ns-_new] -ns-_depth = current -ns-_depth
tag @e[type=area_effect_cloud,tag=-ns-_new] remove -ns-_new

# -debug_anchor-

execute run function -ns-:select_entity
function -ns-:-call_ns-/-call/fn-/-iterate_as-
execute if score breakpoint -ns-_global matches 0 as @e[type=area_effect_cloud,tag=-ns-_function_call] if score @s -ns-_depth = current -ns-_depth run function -ns-:-call_ns-/-call/fn-/return
