# when:
function test:utils/missing_function

# then:
say [test: summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"success"}'}]
