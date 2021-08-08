kill @e[type=sheep,tag=test]
summon sheep ~ ~ ~ {Tags: [test], NoAI: true}
execute as @e[type=sheep,tag=test] at @s anchored eyes run function test:anchored_eyes_nested_function_call/anchored_eyes
