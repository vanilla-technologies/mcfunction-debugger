execute as @e[type=area_effect_cloud,tag=-ns-_selected_entity_marker] if score @s -ns-_depth = current -ns-_depth run tag @s add -ns-_tmp
execute as @e[type=area_effect_cloud,tag=-ns-_tmp,limit=1] run tag @s add -ns-_current
tag @e[type=area_effect_cloud] remove -ns-_tmp

execute as @e[type=area_effect_cloud,tag=-ns-_current] if score @s -ns-_depth = current -ns-_depth run tag @s add -ns-_tmp

execute if score current -ns-_anchor matches 0 at @e[type=area_effect_cloud,tag=-ns-_tmp] anchored feet run function -ns-:-orig_ns-/-orig/fn-/-line_numbers-
execute if score current -ns-_anchor matches 1 at @e[type=area_effect_cloud,tag=-ns-_tmp] anchored eyes run function -ns-:-orig_ns-/-orig/fn-/-line_numbers-

execute if score breakpoint -ns-_global matches 0 as @e[type=area_effect_cloud,tag=-ns-_current] if score @s -ns-_depth = current -ns-_depth run kill @s

execute if score breakpoint -ns-_global matches 0 as @e[type=area_effect_cloud,tag=-ns-_selected_entity_marker] if score @s -ns-_depth = current -ns-_depth run tag @s add -ns-_tmp
execute if score breakpoint -ns-_global matches 0 if entity @e[type=area_effect_cloud,tag=-ns-_tmp] run function -ns-:-orig_ns-/-orig/fn-/iterate_same_executor
