function debug:id/init_self
summon area_effect_cloud ~ ~ ~ {Duration: 2147483647, Tags: [debug_new, debug, debug_selected_entity_marker]}
teleport @e[type=area_effect_cloud,tag=debug_new] ~ ~ ~ ~ ~
scoreboard players operation @e[type=area_effect_cloud,tag=debug_new] debug_id = @s debug_id
scoreboard players operation @e[type=area_effect_cloud,tag=debug_new] debug_depth = current debug_depth
tag @e[type=area_effect_cloud,tag=debug_new] remove debug_new