kill @e[type=area_effect_cloud,tag=test]
summon area_effect_cloud ~ ~ ~ {Duration: 0, Tags: [test]}
function test:function_call_preserves_aec_age_0t_in_server_context/assert
