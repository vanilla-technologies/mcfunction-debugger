kill @e[type=sheep,tag=test]
summon sheep ~ ~ ~ {Tags: [test], NoAI: true}
execute as @e[type=sheep,tag=test] at @s run function test:anchored_default_return/anchored_default
