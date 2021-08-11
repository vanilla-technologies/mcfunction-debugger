execute unless score breakpoint -ns-_global matches 1 run tellraw @a {"text": "Cannot resume, no function is suspended at a breakpoint!","color": "red"}
execute if score breakpoint -ns-_global matches 1 run function -ns-:resume_unchecked
