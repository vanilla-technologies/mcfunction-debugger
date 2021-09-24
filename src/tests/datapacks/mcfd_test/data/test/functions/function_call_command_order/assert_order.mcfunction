execute if entity @s[tag=test_sheep1] unless score test_score test_global matches 1 run say [test: scoreboard players add test_score test_global 0]
execute if entity @s[tag=test_sheep1] if score test_score test_global matches 1 run scoreboard players add test_score test_global 1

execute if entity @s[tag=test_sheep2] unless score test_score test_global matches 3 run say [test: scoreboard players add test_score test_global 0]
execute if entity @s[tag=test_sheep2] if score test_score test_global matches 3 run scoreboard players add test_score test_global 1

execute if entity @s[tag=test_sheep1] unless score test_score test_global matches 2 run say [test: scoreboard players add test_score test_global 0]
execute if entity @s[tag=test_sheep1] if score test_score test_global matches 2 run scoreboard players add test_score test_global 1

execute if entity @s[tag=test_sheep2] unless score test_score test_global matches 4 run say [test: scoreboard players add test_score test_global 0]
execute if entity @s[tag=test_sheep2] if score test_score test_global matches 4 run scoreboard players add test_score test_global 1
