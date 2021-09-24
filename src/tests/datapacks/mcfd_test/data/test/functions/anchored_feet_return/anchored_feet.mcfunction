execute anchored eyes run function test:utils/do_nothing

kill @e[type=area_effect_cloud,tag=test]
summon area_effect_cloud ^ ^ ^ {Tags: ["test"]}
execute store result score aec_y test_global run data get entity @e[type=area_effect_cloud,tag=test,limit=1] Pos[1] 1000
execute store result score sheep_y test_global run data get entity @s Pos[1] 1000
scoreboard players operation diff_y test_global = aec_y test_global
scoreboard players operation diff_y test_global -= sheep_y test_global

say [@: function minect:enable_logging]
execute if score diff_y test_global matches 0 run say [test: tag @s add success]
execute unless score diff_y test_global matches 0 run say [test: scoreboard players add diff_y test_global 0]
say [@: function minect:reset_logging]
