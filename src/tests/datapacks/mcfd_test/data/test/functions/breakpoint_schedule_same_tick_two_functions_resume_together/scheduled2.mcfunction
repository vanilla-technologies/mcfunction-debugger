kill @e[type=area_effect_cloud,tag=test2]
summon area_effect_cloud ~ ~ ~ {Age: 1, Tags: [test2]}
execute store result score gametime2 test_global run time query gametime
