# TODO always call 'iterate'? 'iterate_same_executor'
execute unless score breakpoint -ns-_global matches 1 run function -ns-:-orig_ns-/-orig/fn-/iterate
execute unless score breakpoint -ns-_global matches 1 as @e[type=area_effect_cloud,tag=-ns-_function_call] if score @s -ns-_depth = current -ns-_depth run function -ns-:-orig_ns-/-orig/fn-/return
