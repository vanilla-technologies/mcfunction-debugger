scoreboard players set no_execute test_global 0
function test:function_no_execute/increase_score

say [@: function minect:enable_logging]
execute if score no_execute test_global matches 1 run say [test: tag @s add success]
execute unless score no_execute test_global matches 1 run say [test: scoreboard players add no_execute test_global 0]
say [@: function minect:reset_logging]
