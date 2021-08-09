scoreboard players operation tick_was_breakpoint_before test_global = breakpoint mcfd_global

execute if score breakpoint mcfd_global matches 1 run function -on_breakpoint-
# If breakpoint was hint after age increment, also resume after age increment
execute if score breakpoint mcfd_global matches 1 if score tick_hit_breakpoint test_global matches 1 run function debug:resume
# Otherwise resume before age increment
execute if score breakpoint mcfd_global matches 1 unless score tick_hit_breakpoint test_global matches 1 run schedule function debug:resume 1t

execute if score tick test_global matches 1 run function -test-
scoreboard players reset tick test_global

scoreboard players operation tick_hit_breakpoint test_global = breakpoint mcfd_global
execute if score tick_was_breakpoint_before test_global matches 1 run scoreboard players set tick_hit_breakpoint test_global 0
