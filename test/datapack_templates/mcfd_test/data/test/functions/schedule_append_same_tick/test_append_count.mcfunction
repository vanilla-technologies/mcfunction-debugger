say [@: function minect:enable_logging]
execute if score append test matches 1 run say [test: tag @s add success]
execute unless score append test matches 1 run say [test: tag @s add failure]
say [@: function minect:reset_logging]