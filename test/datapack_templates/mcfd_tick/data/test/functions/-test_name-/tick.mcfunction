execute if score reload_timer minect_global matches 1 if score breakpoint mcfd_global matches 1 run function debug:resume

execute if score tick test_global matches 1 run function -fn-
scoreboard players reset tick test_global
