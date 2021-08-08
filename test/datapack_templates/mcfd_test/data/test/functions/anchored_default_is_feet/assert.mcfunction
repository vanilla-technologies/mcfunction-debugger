kill @e[type=area_effect_cloud,tag=test]
summon area_effect_cloud ^ ^ ^ {Tags: ["test"]}
execute store result score aec_y test_global run data get entity @e[type=area_effect_cloud,tag=test,limit=1] Pos[1]
execute store result score sheep_y test_global run data get entity @s Pos[1]

say [@: function minect:enable_logging]
execute if score aec_y test_global = sheep_y test_global run say [test: tag @s add success]
execute unless score aec_y test_global = sheep_y test_global run say [test: scoreboard players add aec_y test_global 0]
say [@: function minect:reset_logging]
