say [@: function minect:enable_logging]
execute if score test_score test_global matches 1 unless score @e[type=sheep,tag=test_sheep1,limit=1] test_global matches 1 run say [test: scoreboard players add @e[type=sheep,tag=test_sheep1] test_global 0]
execute if score test_score test_global matches 1 if score @e[type=sheep,tag=test_sheep2,limit=1] test_global matches -2147483648..2147483647 run say [test: scoreboard players add @e[type=sheep,tag=test_sheep2] test_global 0]
execute if score test_score test_global matches 2 unless score @e[type=sheep,tag=test_sheep1,limit=1] test_global matches 2 run say [test: scoreboard players add @e[type=sheep,tag=test_sheep1] test_global 0]
execute if score test_score test_global matches 2 unless score @e[type=sheep,tag=test_sheep2,limit=1] test_global matches 1 run say [test: scoreboard players add @e[type=sheep,tag=test_sheep2] test_global 0]
say [@: function minect:reset_logging]

function debug:resume
