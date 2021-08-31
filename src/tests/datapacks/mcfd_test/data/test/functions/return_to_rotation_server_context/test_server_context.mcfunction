kill @e[type=area_effect_cloud,tag=test,tag=rotation1]
summon area_effect_cloud ~ ~ ~ {Tags: [test, rotation1]}
teleport @e[type=area_effect_cloud,tag=rotation1,limit=1] ~ ~ ~ ~ ~
execute store result score aec1_x test_global run data get entity @e[type=area_effect_cloud,tag=rotation1,limit=1] Rotation[0] 1000

kill @e[type=sheep,tag=test]
summon sheep ~3 ~ ~ {Tags: [test], NoAI: true}
execute rotated ~-4 ~ as @e[type=sheep,tag=test] run function test:return_to_rotation_server_context/do_nothing

kill @e[type=area_effect_cloud,tag=test,tag=rotation2]
summon area_effect_cloud ~ ~ ~ {Tags: [test, rotation2]}
teleport @e[type=area_effect_cloud,tag=rotation2,limit=1] ~ ~ ~ ~ ~
execute store result score aec2_x test_global run data get entity @e[type=area_effect_cloud,tag=rotation2,limit=1] Rotation[0] 1000

scoreboard players operation diff_x test_global = aec1_x test_global
scoreboard players operation diff_x test_global -= aec2_x test_global

say [@: function minect:enable_logging]
execute if score diff_x test_global matches 0 run say [test: tag @s add success]
execute unless score diff_x test_global matches 0 run say [test: scoreboard players add diff_x test_global 0]
say [@: function minect:reset_logging]
