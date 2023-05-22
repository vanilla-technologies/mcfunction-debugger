scoreboard players set test_score test_global 1

kill @e[type=sheep,tag=test]
summon sheep ~ ~ ~ {Tags: [test, test_sheep1], NoAI: true}
summon sheep ~2 ~ ~ {Tags: [test, test_sheep2], NoAI: true}

execute as @e[type=sheep,tag=test] run function test:function_call_command_order/assert_order
# If the test gets to here without failing then it's successful
say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"success"}'}]
