execute store result score @s -ns-_Age run data get entity @s Age
execute store result score @s -ns-_Duration run data get entity @s Duration
execute store result score @s -ns-_WaitTime run data get entity @s WaitTime

data modify entity @s Age set value 0
data modify entity @s Duration set value -1
data modify entity @s WaitTime set value -2147483648

tag @s add -ns-_frozen
