kill @e[type=area_effect_cloud,tag=test]
# Not at ~ ~ ~ to have a different position than the server context
summon area_effect_cloud ~5.25 ~ ~ {Tags: [test]}
execute as @e[type=area_effect_cloud,tag=test,limit=1] at @s run function test:return_to_position_entity_context/test_entity_context
