scoreboard players add current namespace_depth 1

summon area_effect_cloud ~ ~ ~ {Duration: 2147483647, Tags: [namespace_new, namespace, namespace_function_call, namespace_caller_namespace_caller_function]}
scoreboard players operation @e[type=area_effect_cloud,tag=namespace_new] namespace_anchor = current namespace_anchor
scoreboard players operation @e[type=area_effect_cloud,tag=namespace_new] namespace_depth = current namespace_depth
tag @e[type=area_effect_cloud,tag=namespace_new] remove namespace_new

# debug_anchor

execute run function namespace:select_entity
# TODO: replace iterate with iterate or iterate_same_executor if no as in execute
function namespace:callee_namespace/callee_function/iterate
execute if score breakpoint namespace_global matches 0 as @e[type=area_effect_cloud,tag=namespace_function_call] if score @s namespace_depth = current namespace_depth run function namespace:callee_namespace/callee_function/return
