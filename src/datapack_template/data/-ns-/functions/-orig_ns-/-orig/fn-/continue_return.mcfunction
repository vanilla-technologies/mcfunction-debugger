execute if score breakpoint -ns-_global matches 0 run function -ns-:-orig_ns-/-orig/fn-/iterate
execute if score breakpoint -ns-_global matches 0 as @e[type=area_effect_cloud,tag=-ns-_function_call] if score @s -ns-_depth = current -ns-_depth run function -ns-:-orig_ns-/-orig/fn-/return
