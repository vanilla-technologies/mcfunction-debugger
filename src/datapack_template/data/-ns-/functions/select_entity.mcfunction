function -ns-:id/init_self
summon area_effect_cloud ~ ~ ~ {Duration: 2147483647, Tags: [-ns-_new, -ns-, -ns-_selected_entity_marker]}
teleport @e[type=area_effect_cloud,tag=-ns-_new] ~ ~ ~ ~ ~
scoreboard players operation @e[type=area_effect_cloud,tag=-ns-_new] -ns-_id = @s -ns-_id
scoreboard players operation @e[type=area_effect_cloud,tag=-ns-_new] -ns-_depth = current -ns-_depth
tag @e[type=area_effect_cloud,tag=-ns-_new] remove -ns-_new
