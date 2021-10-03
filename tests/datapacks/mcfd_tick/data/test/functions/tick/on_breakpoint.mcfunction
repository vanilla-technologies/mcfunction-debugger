# If a test defined an on_breakpoint function don't automatically resume breakpoints
execute store success score on_breakpoint_success test_global run function -on_breakpoint-

# If breakpoint was hint after age increment, also resume after age increment
execute unless score on_breakpoint_success test_global matches 1 unless entity @e[type=area_effect_cloud,tag=mcfd_before_age_increment] run function debug:resume
# Otherwise resume before age increment
execute unless score on_breakpoint_success test_global matches 1 if entity @e[type=area_effect_cloud,tag=mcfd_before_age_increment] run schedule function debug:resume 1t

# Call on_breakpoint function only once per breakpoint
tag @s add test_tick
