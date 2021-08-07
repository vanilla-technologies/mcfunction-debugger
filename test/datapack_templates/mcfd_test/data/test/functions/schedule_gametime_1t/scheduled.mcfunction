execute store result score gametime2 test_global run time query gametime
scoreboard players operation gametime2 test_global -= gametime1 test_global

say [@: function minect:enable_logging]
execute if score gametime2 test_global matches 1 run say [test: tag @s add success]
execute unless score gametime2 test_global matches 1 run say [test: tag @s add failure]
say [@: function minect:reset_logging]
