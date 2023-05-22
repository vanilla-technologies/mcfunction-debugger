kill @e[type=area_effect_cloud,tag=!minect]
# Schedule for 2t because we might kill the before_age_increment marker with the kill above
schedule function test:internals_are_hidden_from_selector_after_schedule/test_internal 2t
