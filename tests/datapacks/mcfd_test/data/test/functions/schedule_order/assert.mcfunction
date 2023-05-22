# multiply is expected to be executed before add
execute if score test_score test_global matches 5 run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"success"}'}]
execute unless score test_score test_global matches 5 run say [test: scoreboard players add test_score test_global 0]
