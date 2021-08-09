execute if score breakpoint mcfd_global matches 1 run function -on_breakpoint-
# If breakpoint was hint after age increment, also resume after age increment
execute if score breakpoint mcfd_global matches 1 unless entity @e[type=area_effect_cloud,tag=mcfd_before_age_increment] run function debug:resume
# Otherwise resume before age increment
execute if score breakpoint mcfd_global matches 1 if entity @e[type=area_effect_cloud,tag=mcfd_before_age_increment] run schedule function debug:resume 1t

execute if score tick test_global matches 1 run function -test-
scoreboard players reset tick test_global
