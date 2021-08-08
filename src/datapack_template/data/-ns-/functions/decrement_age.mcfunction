execute store result score @s -ns-_age run data get entity @s Age
execute if score @s -ns-_age matches 2147483647 run tag @s add -ns-_max_age
execute unless score @s -ns-_age matches 2147483647 run scoreboard players remove @s -ns-_age 1
execute store result entity @s Age int 1 run scoreboard players get @s -ns-_age
