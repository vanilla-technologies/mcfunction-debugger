scoreboard players set @s test_global 0
kill @e[type=sheep,tag=test]
summon sheep ~ ~ ~ {Tags: [test, test_sheep1], NoAI: true}
summon sheep ~10 ~ ~ {Tags: [test, test_sheep2], NoAI: true}
execute at @e[type=sheep,tag=test] run function test:execute_at_entity_context/summon_aec

# Reset is necessary in server context
scoreboard players reset entity_context test_global
scoreboard players operation entity_context test_global = @s test_global
scoreboard players set position_sheep1 test_global 0
scoreboard players set position_sheep2 test_global 0
execute at @e[type=sheep,tag=test_sheep1] as @e[type=area_effect_cloud,tag=sheep_pos,distance=..1] run scoreboard players add position_sheep1 test_global 1
execute at @e[type=sheep,tag=test_sheep2] as @e[type=area_effect_cloud,tag=sheep_pos,distance=..1] run scoreboard players add position_sheep2 test_global 1

say [@: function minect:enable_logging]
execute if score entity_context test_global matches 2 if score position_sheep1 test_global matches 1 if score position_sheep2 test_global matches 1 run say [test: tag @s add success]
execute unless score entity_context test_global matches 2 run say [test: scoreboard players add entity_context test_global 0]
execute unless score position_sheep1 test_global matches 1 run say [test: scoreboard players add position_sheep1 test_global 0]
execute unless score position_sheep2 test_global matches 1 run say [test: scoreboard players add position_sheep2 test_global 0]
say [@: function minect:reset_logging]
