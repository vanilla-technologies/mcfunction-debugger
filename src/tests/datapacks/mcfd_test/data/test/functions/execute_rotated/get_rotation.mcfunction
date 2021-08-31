kill @e[type=area_effect_cloud,tag=test,tag=rotation2]
summon area_effect_cloud ~ ~ ~ {Tags: [test, rotation2]}
teleport @e[type=area_effect_cloud,tag=rotation2,limit=1] ~ ~ ~ ~ ~
execute store result score aec2_x test_global run data get entity @e[type=area_effect_cloud,tag=rotation2,limit=1] Rotation[0] 1000
