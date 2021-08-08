kill @e[type=area_effect_cloud,tag=test]
summon area_effect_cloud ~ ~ ~ {Duration: 11, Tags: [test]}
schedule function test_before_age_increment:schedule_aec_age_10t/scheduled 10t
