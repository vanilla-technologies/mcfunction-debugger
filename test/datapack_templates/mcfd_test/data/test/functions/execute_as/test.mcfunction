kill @e[type=sheep,tag=test]
summon sheep ~ ~ ~ {Tags: [test, test_sheep1], NoAI: 1b}
summon sheep ~4 ~ ~3 {Tags: [test, test_sheep2], NoAI: 1b}
scoreboard players set @e[type=sheep,tag=test] test_global 0
execute as @e[type=sheep,tag=test] run function test:execute_as/increase_score

say [@: function minect:enable_logging]
execute if score @e[type=sheep,tag=test_sheep1,limit=1] test_global matches 1 if score @e[type=sheep,tag=test_sheep2,limit=1] test_global matches 1 run say [test: tag @s add success]
execute unless score @e[type=sheep,tag=test_sheep1,limit=1] test_global matches 1 run say [test: scoreboard players add @e[type=sheep,tag=test_sheep1,limit=1] test_global 0]
execute unless score @e[type=sheep,tag=test_sheep2,limit=1] test_global matches 1 run say [test: scoreboard players add @e[type=sheep,tag=test_sheep2,limit=1] test_global 0]
say [@: function minect:reset_logging]
