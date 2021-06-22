execute if score breakpoint namespace_global matches 0 run function namespace:original_namespace/original_function/iterate
execute if score breakpoint namespace_global matches 0 as @e[type=area_effect_cloud,tag=namespace_function_call] if score @s namespace_depth = current namespace_depth run function namespace:original_namespace/original_function/return
