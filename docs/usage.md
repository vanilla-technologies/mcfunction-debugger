# Usage

1. [Define Breakpoints](#define-breakpoints)
2. [Generate a Debug Datapack](#generate-a-debug-datapack)
3. [Load the Debug Datapack in Minecraft](#load-the-debug-datapack-in-minecraft)
4. [Debug the Datapack in Minecraft](#debug-the-datapack-in-minecraft)
    1. [Start Debugging](#start-debugging)
    2. [Hot Code Replacement](#hot-code-replacement)
    3. [Resume Debugging](#resume-debugging)
    4. [Stop Debugging](#stop-debugging)
5. [Uninstall the Debug Datapack](#uninstall-the-debug-datapack)

## Define Breakpoints

A breakpoint marks a spot in your datapack where execution should stop to enable you to take a look at the state of the Minecraft world and check whether everything is as expected.

A breakpoint is defined by adding the line comment `# breakpoint` to any `*.mcfunction` file. The space after `#` is important, but leading and trailing whitespace is ignored.

For example, imagine you have a datapack called `my_datapack` with a function `example:do_stuff`:

```mcfunction
say Starting
scoreboard objectives add example dummy
scoreboard players set @s example 42
setblock ~1 ~ ~ stone
# breakpoint
say Running
scoreboard objectives remove example
setblock ~ ~ ~1 stone
say Stopping
```

## Generate a Debug Datapack

Once you have added breakpoints to your datapack, you need to generate a debug datapack.
For this you need to [open a command line](how-to-open-a-command-line.md) in the directory that contains the `mcfunction-debugger` binary you downloaded previously (ideally in Minecraft's `saves` directory).
Then (on the command line) you can execute the following command:

```
mcfunction-debugger --input <path to your datapack> --output <path to generate>
```

For example, if the datapack `my_datapack` is in the world `New World`:

```
mcfunction-debugger --input "New World/datapacks/my_datapack" --output "New World/datapacks/debug_my_datapack"
```

The input datapack must be a directory containing a `pack.mcmeta` file.

For more command line options see [here](command-line-interface.md).

## Load the Debug Datapack in Minecraft

To load the debug datapack in Minecraft you can reopen your Minecraft World or simply execute the following Minecraft command:
```
/reload
```

If your datapack contains a `tick.json` (to run functions periodically) you should disable it by executing:
```
/datapack disable "file/my_datapack"
```

When using [shadowing](command-line-interface.md#--shadow): if you have other datapacks that may call a function of the debug datapack in their `tick.json`, it is important that they are enabled before the debug datapack.
You can check this with:
```
/datapack list
```

And if this is not already the case, move the debug datapack to the end with:
```
/datapack disable "file/debug_my_datapack"
/datapack enable "file/debug_my_datapack" last
```

## Debug the Datapack in Minecraft

### Start Debugging

To start a debug session in Minecraft execute the command:
```
/function debug:<your_namespace>/<your_function>
```

For example, to debug the function `example:do_stuff` execute:
```
/function debug:example/do_stuff
```

The function will then execute normally until it reaches a breakpoint:
```
[Info] Started debugging example:do_stuff
[Herobrine] Starting
[Info] Suspended at breakpoint example:do_stuff:5
 To resume run: /function debug:resume
 To stop run: /function debug:stop
Executed 86 commands from function 'debug:example/do_stuff'
```

You can then look around and see that the function `example:do_stuff` placed a stone block to your east.
You can now move around freely.
The debugger remembers the position and rotation from which the function was executed and marks them with green and blue particles respectively.

For more complex datapacks you may want to inspect scoreboard or NBT values, summon or kill entities, etc.
For convenience the scores of the current `@s` entity are displayed in the sidebar on the right.
You can clear the sidebar with:
```
/scoreboard objectives setdisplay sidebar
```

You can restore the sidebar any time with:
```
/function debug:show_scores
```

### Hot Code Replacement

You can add more breakpoints to your datapack while debugging.
For instance you can add a second breakpoint before the `say Stopping` command in line 9 of the function `example:do_stuff`:

```mcfunction
say Starting
scoreboard objectives add example dummy
scoreboard players set @s example 42
setblock ~1 ~ ~ stone
# breakpoint
say Running
scoreboard objectives remove example
setblock ~ ~ ~1 stone
# breakpoint
say Stopping
```

You then need to [regenerate the debug datapack](#generate-a-debug-datapack) and execute `/reload` in Minecraft to [reload the debug datapack](#load-the-debug-datapack-in-minecraft).

You are not limited to adding breakpoints while suspended.
You can add, change or remove breakpoints, commands or even whole functions.
Just ensure that the current breakpoint, as well as all function calls, involved in the current call stack, keep their line number.
Otherwise the debugger will not know where to resume execution.

For example, consider debugging the function `example:foo` of the following datapack:

`example:foo`:
```mcfunction
say Entering example:foo
say Calling function example:bar
function example:bar
say Exiting example:foo
```

`example:bar`:
```mcfunction
say Entering example:bar
# breakpoint
say Exiting example:bar
```

While suspended at the breakpoint, you can change the datapack as you like, as long as the breakpoint in `example:bar` stays at line 2 and the function command in `example:foo` stays at line 3.

### Resume Debugging

To resume the execution click on the command in the message or manually execute:
```
/function debug:resume
```
```
[Info] Resuming debugging from example:do_stuff:5
[Herobrine] Running
[Info] Suspended at breakpoint example:do_stuff:9
 To resume run: /function debug:resume
 To stop run: /function debug:stop
Executed 119 commands from function 'debug:resume'
```

Now a second stone block was placed south of where you started debugging the function.

### Stop Debugging

You can either keep debugging your function until it terminates or abort early by executing:
```
/function debug:stop
```
```
[Info] Debugging session was stopped.
Executed 16 commands from function 'debug:stop'
```

## Uninstall the Debug Datapack

When you [loaded the debug datapack in Minecraft](#load-the-debug-datapack-in-minecraft), it automatically installed itself.
To properly uninstall the datapack you should execute:
```
/function debug:uninstall
```

This will i.a. remove debugger internal scoreboards and disable the debug datapack.
You can then safely delete the debug datapack from disk.

It is important to uninstall the debug datapack when you are finished,
because the debug datapacks will **NOT** work correctly when more than one is enabled at once.

If you change your mind you can simply reenable the debug datapack with:
```
/datapack enable "file/debug_my_datapack"
```
