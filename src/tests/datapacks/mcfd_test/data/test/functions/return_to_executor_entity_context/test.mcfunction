kill @e[type=area_effect_cloud,tag=test]
summon area_effect_cloud ~ ~ ~ {Tags: [test]}
execute as @e[type=area_effect_cloud,tag=test,limit=1] run function test:return_to_executor_entity_context/test_entity_context
