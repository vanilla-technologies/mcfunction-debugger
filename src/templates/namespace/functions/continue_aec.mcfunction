scoreboard players set continue_success namespace_global 0

# continue_cases

execute if score continue_success namespace_global matches 0 run tellraw @a [{"text": "Breakpoint ","color": "red"},{"selector":"@s","color": "red"},{"text": " was deleted","color": "red"}]
execute unless score continue_success namespace_global matches 0 run kill @s