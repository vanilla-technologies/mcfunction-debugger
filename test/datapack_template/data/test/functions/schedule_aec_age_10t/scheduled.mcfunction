execute store result score aec_age test_time run data get entity @e[type=area_effect_cloud,tag=test,limit=1] Age

say [@: function minect:enable_logging]
execute if score aec_age test_time matches 10 run say [test: tag @s add success]
execute unless score aec_age test_time matches 10 run say [test: tag @s add failure]
say [@: function minect:reset_logging]
kill @e[type=area_effect_cloud,tag=test]
