execute unless entity @e[type=area_effect_cloud,tag=!minect] run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"success"}'}]
execute if entity @e[type=area_effect_cloud,tag=!minect] run say [test: tag @e[type=area_effect_cloud,tag=!minect] add failure]
