execute if score breakpoint mcfd_global matches 1 run function -on_breakpoint-
execute if score breakpoint mcfd_global matches 1 run function debug:resume

execute if score tick test_global matches 1 run function -test-
scoreboard players reset tick test_global
