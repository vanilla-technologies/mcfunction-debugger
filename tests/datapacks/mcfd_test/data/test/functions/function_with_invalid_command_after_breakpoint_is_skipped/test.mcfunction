# given:
scoreboard players set test_score test_global 0

# when:
function test:utils/add_1_invalid_after_breakpoint

# then:
say [@: function minect:enable_logging]
execute if score test_score test_global matches 0 run say [test: tag @s add success]
execute unless score test_score test_global matches 0 run say [test: scoreboard players add test_score test_global 0]
say [@: function minect:reset_logging]
