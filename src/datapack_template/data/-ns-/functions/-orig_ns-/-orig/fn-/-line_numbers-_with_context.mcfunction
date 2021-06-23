execute unless score @s -ns-_id matches 0 run scoreboard players operation @e[tag=!-ns-_selected_entity_marker] -ns-_id -= @s -ns-_id
execute unless score @s -ns-_id matches 0 as @e[tag=!-ns-_selected_entity_marker,scores={-ns-_id=0}] run tag @s add -ns-_tmp
execute unless score @s -ns-_id matches 0 run scoreboard players operation @e[tag=!-ns-_selected_entity_marker] -ns-_id += @s -ns-_id

execute unless score @s -ns-_id matches 0 unless entity @e[tag=!-ns-_selected_entity_marker,tag=-ns-_tmp] run say Error: selected entity was killed
execute if score @s -ns-_id matches 0 at @s run schedule function -ns-:-orig_ns-/-orig/fn-/-line_numbers- 1t
execute if score current -ns-_anchor matches 0 at @s as @e[tag=!-ns-_selected_entity_marker,tag=-ns-_tmp] anchored feet run function -ns-:-orig_ns-/-orig/fn-/-line_numbers-
execute if score current -ns-_anchor matches 1 at @s as @e[tag=!-ns-_selected_entity_marker,tag=-ns-_tmp] anchored eyes run function -ns-:-orig_ns-/-orig/fn-/-line_numbers-

execute if score breakpoint -ns-_global matches 0 run kill @s
