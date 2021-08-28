kill @e[type=sheep,tag=test]
summon sheep ~ ~ ~ {Tags: [test], NoAI: true}
execute as @e[type=sheep,tag=test] run function test:return_to_executor_server_context/do_nothing

say [@: function minect:enable_logging]
execute unless entity @s run say [test: tag @s add success]
execute if entity @s run say [test: tag @s add failure]
say [@: function minect:reset_logging]
