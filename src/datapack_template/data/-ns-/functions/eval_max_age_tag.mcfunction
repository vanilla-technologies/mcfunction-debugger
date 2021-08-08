execute if entity @s[tag=-ns-_max_age] run tag @s add -ns-_tmp
tag @s remove -ns-_max_age
execute if entity @s[tag=-ns-_tmp] run function -ns-:decrement_age
tag @s remove -ns-_tmp

