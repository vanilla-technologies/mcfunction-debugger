say [@: function minect:enable_logging]
execute unless entity @e[type=area_effect_cloud,tag=test] run say [test: tag @s add success]
execute if entity @e[type=area_effect_cloud,tag=test] run say [test: tag @s add failure]
say [@: function minect:reset_logging]

kill @e[type=area_effect_cloud,tag=test]
