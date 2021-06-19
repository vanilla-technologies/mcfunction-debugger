execute as @e[type=area_effect_cloud,tag=namespace_selected_entity_marker] if score @s namespace_depth = current namespace_depth run tag @s add namespace_tmp
execute as @e[type=area_effect_cloud,tag=namespace_tmp,limit=1] run tag @s add namespace_current
tag @e[type=area_effect_cloud] remove namespace_tmp

execute as @e[type=area_effect_cloud,tag=namespace_current] if score @s namespace_depth = current namespace_depth run tag @s add namespace_tmp

execute if score current namespace_anchor matches 0 at @e[type=area_effect_cloud,tag=namespace_tmp] anchored feet run function namespace:original_namespace/original_function/line_numbers
execute if score current namespace_anchor matches 1 at @e[type=area_effect_cloud,tag=namespace_tmp] anchored eyes run function namespace:original_namespace/original_function/line_numbers

execute if score breakpoint namespace_breakpoint matches 0 as @e[type=area_effect_cloud,tag=namespace_current] if score @s namespace_depth = current namespace_depth run kill @s

execute if score breakpoint namespace_breakpoint matches 0 as @e[type=area_effect_cloud,tag=namespace_selected_entity_marker] if score @s namespace_depth = current namespace_depth run tag @s add namespace_tmp
execute if score breakpoint namespace_breakpoint matches 0 if entity @e[type=area_effect_cloud,tag=namespace_tmp] run function namespace:original_namespace/original_function/iterate_same_executor
