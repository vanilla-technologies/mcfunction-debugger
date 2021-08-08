kill @e[type=area_effect_cloud,tag=test]
summon area_effect_cloud ~ ~ ~ {Age: 2147483647, Duration: 2147483647, Tags: [test]}
schedule function test_before_age_increment:schedule_aec_age_1t_max_age/scheduled 1t
