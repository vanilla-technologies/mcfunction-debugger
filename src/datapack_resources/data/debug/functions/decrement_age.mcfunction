execute store result score @s debug_age run data get entity @s Age
execute if score @s debug_age matches -2147483648 run tag @s add debug_min_age
execute unless score @s debug_age matches -2147483648 run scoreboard players remove @s debug_age 1
execute store result entity @s Age int 1 run scoreboard players get @s debug_age
