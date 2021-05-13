scoreboard players add debug_depth debug_depth 1

summon area_effect_cloud ~ ~ ~ {Duration: 2147483647, Tags: [debug_new, debug_function_call, debug_caller_main]}
scoreboard players operation @e[type=area_effect_cloud,tag=debug_new] debug_depth = debug_depth debug_depth
tag @e[type=area_effect_cloud,tag=debug_new] remove debug_new
