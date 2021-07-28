# TODO
## MVP
* Add tag=!debug to all selectors
* disable original datapack in load.json
* enable original datapack when unloaded. Since there is no unload.json we could have a command block at 0/0/0 trying to remove itself and enable orig each tick unless the datapack is loaded.

### Tests
* schedule tests after increment age

* schedule of two functions: if beakpoint, the second schedule is delayed
* schedule function twice for the same tick (append), the function may only be executed once
* schedule replace - only second schedule is executed
* schedule clear
* Bug: returning into server context takes one tick but does not decrement age

* function call without execute
* execute as function visits all selected entities exacly once and changes executor
* execute at function visits all selected positions exacly once and keeps entity executor
* execute at function visits all selected positions exacly once and keeps server executor
* execute rotated function changes rotation
* execute in function changes dimension?
* returning into entity context restores
  * executor
  * position
  * rotation
  * dimension?
* returning into server context restores
  * executor
  * position
  * rotation
  * dimension?
* resuming from breakpoint into entity context restores
  * executor
  * position
  * rotation
  * dimension?

* execute anchored eyes function changes anchor
* execute anchored feet function changes anchor
* default is anchored feet
* anchored eyes propagates to nested function call
* default anchor is restored when returning from function call with anchor eyes
* anchor eyes is restored when returning from function call with anchor feet
* anchor feet is restored when returning from function call with anchor eyes

* after a schedule: no aec is visible to the program
* inside execute function call: no aec is visible to the program

* test various breakpoints

## Advanced
* simpler template for schedule replace without execute
* Selector parser: add support for ']' in strings and NBT, support for player name and UUID, support for limits on amount and type
* preserve load and tick.json (and block/item tags etc.) of original datapack
* Support multiple input datapacks
* Support execute store
* Support function tags
* Look at all commands
* Random tick speed
* Support AECs with Age >= Duration+WaitTime in schedules running at the beginning of the "next tick" after a breakpoint
* Restore command block context by changing block content when continuing from a breakpoint

## Documentation
* Document potential problems with chunkloading
* Document how to increase max command limit
* Document that killing entities while they are executing a function cannot properly be handled by the debugger
  * When restoring the context: if the executing entity is not found: execute unless score @s id matches 0 unless entity @e[tag=!debug_selected_entity_marker,tag=debug_tmp] run say error entity killed while selected

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
