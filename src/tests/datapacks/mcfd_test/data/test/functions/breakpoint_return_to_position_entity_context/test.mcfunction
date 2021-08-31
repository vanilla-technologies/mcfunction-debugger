kill @e[type=area_effect_cloud,tag=test]
summon area_effect_cloud ~ ~ ~ {Tags: [test]}
# Not at ~ ~ ~ to have a different position than the server context
execute positioned ~5.25 ~ ~ as @e[type=area_effect_cloud,tag=test,limit=1] run function test:breakpoint_return_to_position_entity_context/test_entity_context
