execute store result score gametime2 test_global run time query gametime
scoreboard players operation gametime2 test_global -= gametime1 test_global

execute if score gametime2 test_global matches 10 run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"success"}'}]
execute unless score gametime2 test_global matches 10 run say [test: scoreboard players add gametime2 test_global 0]
