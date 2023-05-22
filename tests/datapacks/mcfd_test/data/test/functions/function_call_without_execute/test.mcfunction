scoreboard players set test_score test_global 0
function test:function_call_without_execute/increase_score

execute if score test_score test_global matches 1 run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"success"}'}]
execute unless score test_score test_global matches 1 run say [test: scoreboard players add test_score test_global 0]
