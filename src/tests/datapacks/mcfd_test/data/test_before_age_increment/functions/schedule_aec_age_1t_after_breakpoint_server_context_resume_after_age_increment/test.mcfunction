kill @e[type=area_effect_cloud,tag=test]
summon area_effect_cloud ~ ~ ~ {Duration: 2, Tags: [test]}
# breakpoint
schedule function test_before_age_increment:schedule_aec_age_1t_after_breakpoint_server_context_resume_after_age_increment/scheduled 1t
