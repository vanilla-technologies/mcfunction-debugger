execute store result score @s -ns-_tmp run data get entity @s Age
scoreboard players remove @s -ns-_tmp 1
execute store result entity @s Age int 1 run scoreboard players get @s -ns-_tmp
