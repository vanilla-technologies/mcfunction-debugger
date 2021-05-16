scoreboard players operation @e[tag=!namespace_selected_entity_marker] id -= @s id
execute as @e[tag=!namespace_selected_entity_marker,scores={id=0}] run tag @s add namespace_tmp
scoreboard players operation @e[tag=!namespace_selected_entity_marker] id += @s id

execute if score current namespace_anchor matches 0 at @s as @e[tag=!namespace_selected_entity_marker,tag=namespace_tmp] anchored feet run function namespace:original_namespace/original_function/line_numbers
execute if score current namespace_anchor matches 1 at @s as @e[tag=!namespace_selected_entity_marker,tag=namespace_tmp] anchored eyes run function namespace:original_namespace/original_function/line_numbers

execute if score breakpoint namespace_breakpoint matches 0 run kill @s
