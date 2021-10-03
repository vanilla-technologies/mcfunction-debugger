# breakpoint

kill @e[type=area_effect_cloud,tag=test1]
summon area_effect_cloud ~ ~ ~ {Age: 1, Tags: [test1]}
execute store result score gametime1 test_global run time query gametime
