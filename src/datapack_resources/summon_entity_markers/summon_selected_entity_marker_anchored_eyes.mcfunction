function debug:id/init_self
summon area_effect_cloud ~ ~ ~ {Duration: 2147483647, Tags: [debug_new, debug_selected_entity_marker, debug_anchored_eyes]}
teleport @e[type=area_effect_cloud,tag=debug_new] ~ ~ ~ ~ ~
scoreboard players operation @e[type=area_effect_cloud,tag=debug_new] id = @s id
scoreboard players operation @e[type=area_effect_cloud,tag=debug_new] depth = depth depth
tag @e[type=area_effect_cloud,tag=debug_new] remove debug_new
