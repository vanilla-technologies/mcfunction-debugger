execute as @e[type=area_effect_cloud,tag=-ns-_selected_entity_marker] if score @s -ns-_depth = current -ns-_depth run tag @s add -ns-_tmp
execute as @e[type=area_effect_cloud,tag=-ns-_tmp,limit=1] run tag @s add -ns-_current
tag @e[type=area_effect_cloud] remove -ns-_tmp
execute as @e[type=area_effect_cloud,tag=-ns-_current] if score @s -ns-_depth = current -ns-_depth run function -ns-:-orig_ns-/-orig/fn-/iteration_step
