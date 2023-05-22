scoreboard players set @s test_global 1
kill @e[type=sheep,tag=test]
summon sheep ~ ~ ~ {Tags: [test], NoAI: true}
execute as @e[type=sheep,tag=test] run function test:utils/do_nothing
scoreboard players add @s test_global 1

# Reset is necessary in server context
scoreboard players reset test_score test_global
scoreboard players operation test_score test_global = @s test_global

execute if score test_score test_global matches 2 run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"success"}'}]
execute unless score test_score test_global matches 2 run say [test: scoreboard players add test_score test_global 0]
