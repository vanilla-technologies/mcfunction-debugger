scoreboard players set test_score test_global 0

kill @e[type=sheep,tag=test]
summon sheep ~ ~ ~ {Tags: [test, test_sheep1], NoAI: true}
summon sheep ~4 ~ ~3 {Tags: [test, test_sheep2], NoAI: true}
execute as @e[type=sheep,tag=test] run function test:breakpoint_iterate_selector/increase_score

execute if score @e[type=sheep,tag=test_sheep2,limit=1] test_global matches 2 run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"success"}'}]
execute unless score @e[type=sheep,tag=test_sheep2,limit=1] test_global matches 2 run say [test: scoreboard players add @e[type=sheep,tag=test_sheep2] test_global 0]
