# TODO
* Support execute facing
* Support execute store
* Prevent AECs from counting down their duration
* Support scheduled
* Support function tags
* Look at all commands
* Add tag=!debug to all selectors
* Random tick speed
* Document potential problems with chunkloading
* Document how to increase max command limit
* Support function calls without execute.
* Use commands.json for parsing execute

# Optimizations
* Function calls without execute do not need to store their own context, maybe don't increment the depth counter or something like that.
* If a function call tree does not contain a breakpoint then we can call the original function there.

# Done
* Test effect of order of execute anchored -> Order does not matter
* Can teleport @s change the context position? -> No
