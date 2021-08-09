# Needed when continuing from a breakpoint
execute if score breakpoint -ns-_global matches 1 as @e[type=area_effect_cloud] run function -ns-:unfreeze_aec
scoreboard players set breakpoint -ns-_global 0

tag @s remove -ns-_tmp
tag @e[type=area_effect_cloud] remove -ns-_tmp

# -content-
