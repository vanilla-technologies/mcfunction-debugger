# breakpoint

execute unless entity @s run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"success"}'}]
execute if entity @s run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"failure"}'}]
