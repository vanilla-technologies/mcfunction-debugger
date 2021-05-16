scoreboard players set breakpoint namespace_breakpoint 0
scoreboard players set current namespace_depth 0
scoreboard players set current namespace_anchor 0

function namespace:select_entity
tag @e[type=area_effect_cloud,tag=namespace_selected_entity_marker,scores={namespace_depth=0}] add namespace_current
function namespace:original_namespace/original_function/line_numbers
