execute if score breakpoint mcfd_global matches 1 as @e[type=area_effect_cloud,tag=mcfd_breakpoint,tag=!test_tick] run function test:tick/on_breakpoint

execute if score tick test_global matches 1 run function -test-
scoreboard players reset tick test_global
