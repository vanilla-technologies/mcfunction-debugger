kill @e[type=area_effect_cloud,tag=test]
summon area_effect_cloud ~ ~ ~ {Duration: 1, Tags: [test]}
# breakpoint
schedule function test_after_age_increment:schedule_aec_age_1t_after_breakpoint_server_ctx_resume_before_age_inc/scheduled 1t