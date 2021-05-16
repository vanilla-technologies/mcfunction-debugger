# Needed when continuing from a breakpoint
scoreboard players set breakpoint namespace_breakpoint 0

execute as @e[type=area_effect_cloud,tag=namespace_selected_entity_marker,tag=namespace_current] if score @s namespace_depth = current namespace_depth run function namespace:original_namespace/original_function/line_numbers_with_context
