execute as @e[type=area_effect_cloud,tag=namespace_selected_entity_marker] if score @s namespace_depth = current namespace_depth run tag @s add namespace_tmp
execute as @e[type=area_effect_cloud,tag=namespace_tmp,limit=1] run tag @s add namespace_current
tag @e[type=area_effect_cloud] remove namespace_tmp
execute as @e[type=area_effect_cloud,tag=namespace_current] if score @s namespace_depth = current namespace_depth run function namespace:original_namespace/original_function/iteration_step
