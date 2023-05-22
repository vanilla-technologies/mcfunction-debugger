summon area_effect_cloud ~ ~ ~ {Tags: [test, rotation1]}
teleport @e[type=area_effect_cloud,tag=rotation1,limit=1] ~ ~ ~ ~ ~
execute store result score aec1_x test_global run data get entity @e[type=area_effect_cloud,tag=rotation1,limit=1] Rotation[0] 1000

kill @e[type=sheep,tag=test]
summon sheep ~4 ~ ~3 {Tags: [test, test_sheep2], NoAI: true}
scoreboard players set @e[type=sheep,tag=test] test_global 0
execute rotated ~5.25 ~ as @e[type=sheep,tag=test] run function test:execute_rotated/get_rotation

scoreboard players operation diff_x test_global = aec1_x test_global
scoreboard players operation diff_x test_global -= aec2_x test_global

execute if score diff_x test_global matches -5250 run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"success"}'}]
execute unless score diff_x test_global matches -5250 run say [test: scoreboard players add diff_x test_global 0]
