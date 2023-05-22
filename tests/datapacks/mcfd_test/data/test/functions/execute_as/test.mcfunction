kill @e[type=sheep,tag=test]
summon sheep ~ ~ ~ {Tags: [test, test_sheep1], NoAI: true}
summon sheep ~4 ~ ~3 {Tags: [test, test_sheep2], NoAI: true}
scoreboard players set @e[type=sheep,tag=test] test_global 0
execute as @e[type=sheep,tag=test] run function test:execute_as/increase_score

execute if score @e[type=sheep,tag=test_sheep1,limit=1] test_global matches 1 if score @e[type=sheep,tag=test_sheep2,limit=1] test_global matches 1 run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"success"}'}]
execute unless score @e[type=sheep,tag=test_sheep1,limit=1] test_global matches 1 run say [test: scoreboard players add @e[type=sheep,tag=test_sheep1,limit=1] test_global 0]
execute unless score @e[type=sheep,tag=test_sheep2,limit=1] test_global matches 1 run say [test: scoreboard players add @e[type=sheep,tag=test_sheep2,limit=1] test_global 0]
