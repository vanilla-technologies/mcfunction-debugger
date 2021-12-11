# when:
function test:utils/missing_function

# then:
say [@: function minect:enable_logging]
say [test: tag @s add success]
say [@: function minect:reset_logging]
