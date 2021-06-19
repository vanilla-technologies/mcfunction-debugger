execute unless score @s namespace_id matches 0 run scoreboard players operation @e[tag=!namespace_selected_entity_marker] namespace_id -= @s namespace_id
execute unless score @s namespace_id matches 0 as @e[tag=!namespace_selected_entity_marker,scores={namespace_id=0}] run tag @s add namespace_tmp
execute unless score @s namespace_id matches 0 run scoreboard players operation @e[tag=!namespace_selected_entity_marker] namespace_id += @s namespace_id

execute unless score @s namespace_id matches 0 unless entity @e[tag=!namespace_selected_entity_marker,tag=namespace_tmp] run say Error: selected entity was killed
execute if score @s namespace_id matches 0 at @s run schedule function namespace:original_namespace/original_function/line_numbers 1t
execute if score current namespace_anchor matches 0 at @s as @e[tag=!namespace_selected_entity_marker,tag=namespace_tmp] anchored feet run function namespace:original_namespace/original_function/line_numbers
execute if score current namespace_anchor matches 1 at @s as @e[tag=!namespace_selected_entity_marker,tag=namespace_tmp] anchored eyes run function namespace:original_namespace/original_function/line_numbers

execute if score breakpoint namespace_breakpoint matches 0 run kill @s
