kill @e[type=sheep,tag=test]
summon sheep ~ ~ ~ {Tags: [test], NoAI: true}
summon sheep ~ ~ ~ {Tags: [test], NoAI: true}
scoreboard players set aec_count test_global 0
execute as @e[type=sheep,tag=test] run function test:internals_are_hidden_from_selector_inside_function/count_aecs

say [@: function minect:enable_logging]
execute if score aec_count test_global matches 0 run say [test: tag @s add success]
execute unless score aec_count test_global matches 0 run say [test: scoreboard players add aec_count test_global 0]
say [@: function minect:reset_logging]
