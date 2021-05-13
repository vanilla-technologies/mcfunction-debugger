* Support function calls without execute.
* Use commands.json for parsing execute

# Optimizations
* Function calls without execute do not need to store their own context, maybe don't increment the depth counter or something like that.
* If a function call tree does not contain a breakpoint then we can call the original function there.
