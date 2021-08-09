# Needed when continuing from a breakpoint
execute as @e[type=area_effect_cloud,tag=-ns-_frozen] run function -ns-:unfreeze_aec

tag @s remove -ns-_tmp
tag @e[type=area_effect_cloud] remove -ns-_tmp

# -content-
