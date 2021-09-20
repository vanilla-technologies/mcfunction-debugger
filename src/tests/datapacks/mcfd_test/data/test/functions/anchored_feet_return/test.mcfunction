kill @e[type=sheep,tag=test]
summon sheep ~ ~ ~ {Tags: [test], NoAI: true}
execute as @e[type=sheep,tag=test] at @s anchored feet run function test:anchored_feet_return/anchored_feet
