execute store result score gametime2 test_time run time query gametime
scoreboard players operation gametime2 test_time -= gametime1 test_time

say [@: function minect:enable_logging]
execute if score gametime2 test_time matches 1 run say [test: tag @s add success]
execute unless score gametime2 test_time matches 1 run say [test: tag @s add failure]
say [@: function minect:reset_logging]
