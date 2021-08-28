kill @e[type=area_effect_cloud,tag=test,tag=position1]
summon area_effect_cloud ~ ~ ~ {Tags: [test, position1]}
execute store result score aec1_x test_global run data get entity @e[type=area_effect_cloud,tag=position1,limit=1] Pos[0] 1000

# breakpoint

kill @e[type=area_effect_cloud,tag=test,tag=position2]
summon area_effect_cloud ~ ~ ~ {Tags: [test, position2]}
execute store result score aec2_x test_global run data get entity @e[type=area_effect_cloud,tag=position2,limit=1] Pos[0] 1000

scoreboard players operation diff_x test_global = aec1_x test_global
scoreboard players operation diff_x test_global -= aec2_x test_global

say [@: function minect:enable_logging]
execute if score diff_x test_global matches 0 run say [test: tag @s add success]
execute unless score diff_x test_global matches 0 run say [test: scoreboard players add diff_x test_global 0]
say [@: function minect:reset_logging]
