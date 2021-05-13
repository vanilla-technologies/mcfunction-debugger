execute as @e[type=area_effect_cloud,tag=debug_selected_entity_marker] if score @s debug_depth = debug_depth debug_depth run tag @s add debug_tmp
execute as @e[type=area_effect_cloud,tag=debug_tmp,limit=1] run tag @s add debug_current
tag @e[type=area_effect_cloud] remove debug_tmp