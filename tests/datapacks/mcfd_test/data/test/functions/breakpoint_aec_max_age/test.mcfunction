kill @e[type=area_effect_cloud,tag=test]
summon area_effect_cloud ~ ~ ~ {Age: 2147483647, Duration: 2147483647, Tags: [test]}

# breakpoint

execute store result score aec_age test_global run data get entity @e[type=area_effect_cloud,tag=test,limit=1] Age

execute if entity @e[type=area_effect_cloud,tag=test,limit=1] if score aec_age test_global matches 2147483647 run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"success"}'}]
execute unless entity @e[type=area_effect_cloud,tag=test,limit=1] run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"entity_missing"}'}]
execute unless score aec_age test_global matches 2147483647 run say [test: scoreboard players add aec_age test_global 0]
