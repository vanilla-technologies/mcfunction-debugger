execute unless entity @e[type=area_effect_cloud,tag=test] run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"success"}'}]
execute if entity @e[type=area_effect_cloud,tag=test] run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"entity_exists"}'}]
