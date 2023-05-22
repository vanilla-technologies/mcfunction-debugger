kill @e[type=sheep,tag=test]
summon sheep ~ ~ ~ {Tags: [test], NoAI: true}
execute as @e[type=sheep,tag=test] run function test:utils/do_nothing

execute unless entity @s run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"success"}'}]
execute if entity @s run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"failure"}'}]
