kill @e[type=area_effect_cloud,tag=test_context]
summon area_effect_cloud ~ ~ ~ {Tags: [test_context]}
execute as @e[type=area_effect_cloud,tag=test_context] run function test_after_age_increment:schedule_gametime_1t_after_breakpoint_entity_context_resume_before_age_increment/test_entity_context
