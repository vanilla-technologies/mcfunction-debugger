scoreboard players set @s test_global 1
kill @e[type=sheep,tag=test]
summon sheep ~ ~ ~ {Tags: [test], NoAI: true}
execute as @e[type=sheep,tag=test] run function test:do_nothing
scoreboard players add @s test_global 1

# Reset is necessary in server context
scoreboard players reset test_score test_global
scoreboard players operation test_score test_global = @s test_global

say [@: function minect:enable_logging]
execute if score test_score test_global matches 2 run say [test: tag @s add success]
execute unless score test_score test_global matches 2 run say [test: scoreboard players add test_score test_global 0]
say [@: function minect:reset_logging]
