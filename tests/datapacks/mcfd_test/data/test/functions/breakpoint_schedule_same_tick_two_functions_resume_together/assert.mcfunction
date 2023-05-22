execute store result score age1 test_global run data get entity @e[type=area_effect_cloud,tag=test1,limit=1] Age
execute store result score age2 test_global run data get entity @e[type=area_effect_cloud,tag=test2,limit=1] Age
scoreboard players operation age_diff test_global = age1 test_global
scoreboard players operation age_diff test_global -= age2 test_global

scoreboard players operation gametime_diff test_global = gametime1 test_global
scoreboard players operation gametime_diff test_global -= gametime2 test_global

execute if score age_diff test_global matches 0 if score gametime_diff test_global matches 0 run say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"success"}'}]
execute unless score age_diff test_global matches 0 run say [test: scoreboard players add age_diff test_global 0]
execute unless score gametime_diff test_global matches 0 run say [test: scoreboard players add gametime_diff test_global 0]
