execute store result entity @s Age int 1 run scoreboard players get @s -ns-_Age
execute store result entity @s Duration int 1 run scoreboard players get @s -ns-_Duration
execute store result entity @s WaitTime int 1 run scoreboard players get @s -ns-_WaitTime

tag @s remove -ns-_frozen
