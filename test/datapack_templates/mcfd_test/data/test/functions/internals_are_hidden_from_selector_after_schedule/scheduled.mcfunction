say [@: function minect:enable_logging]
execute unless entity @e[type=area_effect_cloud,tag=!minect_connection] run say [test: tag @s add success]
execute if entity @e[type=area_effect_cloud,tag=!minect_connection] run say [test: tag @e[type=area_effect_cloud,tag=!minect_connection] add failure]
say [@: function minect:reset_logging]
