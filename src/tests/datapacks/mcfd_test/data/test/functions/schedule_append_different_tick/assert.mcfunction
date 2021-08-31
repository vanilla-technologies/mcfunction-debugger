say [@: function minect:enable_logging]
execute if score schedule_append test_global matches 2 run say [test: tag @s add success]
execute unless score schedule_append test_global matches 2 run say [test: scoreboard players add schedule_append test_global 0]
say [@: function minect:reset_logging]
