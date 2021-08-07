execute store result score aec_age test_global run data get entity @e[type=area_effect_cloud,tag=test,limit=1] Age

say [@: function minect:enable_logging]
execute if score aec_age test_global matches -2147483647 run say [test: tag @s add success]
execute unless score aec_age test_global matches -2147483647 run say [test: scoreboard players add aec_age test_global 0]
say [@: function minect:reset_logging]
kill @e[type=area_effect_cloud,tag=test]

