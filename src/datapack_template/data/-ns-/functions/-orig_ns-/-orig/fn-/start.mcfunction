scoreboard players set current -ns-_depth 0
scoreboard players set current -ns-_anchor 0

function -ns-:select_entity
tag @e[type=area_effect_cloud,tag=-ns-_selected_entity_marker,scores={-ns-_depth=0}] add -ns-_current
function -ns-:-orig_ns-/-orig/fn-/-line_numbers-
execute if score breakpoint -ns-_global matches 0 as @e[type=area_effect_cloud,tag=-ns-_selected_entity_marker] if score @s -ns-g_depth = current -ns-_depth run kill @s
