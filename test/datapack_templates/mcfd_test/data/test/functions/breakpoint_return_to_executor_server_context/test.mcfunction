# breakpoint

say [@: function minect:enable_logging]
execute unless entity @s run say [test: tag @s add success]
execute if entity @s run say [test: tag @s add failure]
say [@: function minect:reset_logging]
