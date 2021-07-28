# Optimizations
* Function calls without execute do not need to store their own context, maybe don't increment the depth counter or something like that.
* If a function call tree does not contain a breakpoint then we can call the original function there.
* Summon call stack entities only after hitting a breakpoint. This could allow command blocks calling debug functions inside executes.
* Writing the debug datapack as a zip file could be much faster than creating hundreds of files

# Done
* When trying to start a new function while standing on a breakpoint abort and inform the user
* Test effect of order of execute anchored -> Order does not matter
* Can teleport @s change the context position? -> No
* Anchored eyes must be kept for functions called by a function which is called with anchored eyes
* Support execute facing
* Use commands.json for parsing execute
* Support function calls without execute.
* Store current breakpoint via tags/scoreboard and offer a single function to continue from the current breakpoint
* Support breakpoints in functions executed by the server/command block
* Prevent AECs from counting down their duration
* Support scheduled
* Instead of /start shadow names of original datapack (easier to understand for users and command blocks may still work when not using execute)
* Support AECs with Age >= Duration+WaitTime in schedules running at the beginning of the "next tick" after a breakpoint
* When restoring the context: if the executing entity is not found: execute unless score @s id matches 0 unless entity @e[tag=!debug_selected_entity_marker,tag=debug_tmp] run say error entity killed while selected
