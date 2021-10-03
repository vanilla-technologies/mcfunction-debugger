say [@: function minect:enable_logging]
# multiply is expected to be executed before add
execute if score test_score test_global matches 5 run say [test: tag @s add success]
execute unless score test_score test_global matches 5 run say [test: scoreboard players add test_score test_global 0]
say [@: function minect:reset_logging]
