[![Minecraft: Java Edition 1.14.1 - 1.19.4](https://img.shields.io/badge/Minecraft%3A%20Java%20Edition-1.14.1%20--%201.19.4-informational)](https://www.minecraft.net/store/minecraft-java-edition)
![Minecraft: Bedrock Edition unsupported](https://img.shields.io/badge/Minecraft%3A%20Bedrock%20Edition-unsupported-critical)\
[![crates.io](https://img.shields.io/crates/v/mcfunction-debugger)](https://crates.io/crates/mcfunction-debugger)
[![Build Status](https://img.shields.io/github/actions/workflow/status/vanilla-technologies/mcfunction-debugger/rust.yml?branch=main)](https://github.com/vanilla-technologies/mcfunction-debugger/actions/workflows/rust.yml)

# McFunction-Debugger

McFunction-Debugger is a debugger for Minecraft's *.mcfunction files that does not require any Minecraft mods.

This documentation covers using the debugger via command line. The corresponding [Visual Studio Code](https://code.visualstudio.com/) extension can be found here: https://github.com/vanilla-technologies/mcfunction-debugger-vscode

McFunction-Debugger implements the [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/) to allow easy integration with different IDEs such as Eclipse or Vim (see the [list of supporting IDEs](https://microsoft.github.io/debug-adapter-protocol/implementors/tools/)). If you would like to implement such an integration, you can find documentation in the [mcfunction-debug-adapter](mcfunction-debug-adapter/README.md) directory.

## Usage

You can debug any datapack with the following five steps:

1. Add `# breakpoint` lines in your *.mcfunction files
2. Generate a debug datapack
3. Load the debug datapack in Minecraft
4. Start debugging any of your functions with: `/function debug:<your_namespace>/<your_function>`
5. When finished, uninstall the debug datapack with: `/function debug:uninstall`

A more detailed description can be found [here](docs/usage.md).

## Installation

### Using precompiled binaries

Precompiled binaries are available under [releases](https://github.com/vanilla-technologies/mcfunction-debugger/releases).
We recommend saving the `mcfunction-debugger` binary to Minecraft's `saves` directory for ease of use.
On Windows this is located at `%APPDATA%\.minecraft\saves`.

### Installing from source

McFunction-Debugger is written in Rust so to build it from source you need to [install Rust](https://www.rust-lang.org/tools/install).

You can then install it from [crates.io](https://crates.io/crates/mcfunction-debugger) by running:
```
cargo install mcfunction-debugger
```

Or from GitHub by running:
```
cargo install --git https://github.com/vanilla-technologies/mcfunction-debugger.git
```

To uninstall run:
```
cargo uninstall mcfunction-debugger
```

## Planned features

These features are planned, but not yet implemented:

* Support function tags [#12](https://github.com/vanilla-technologies/mcfunction-debugger/issues/12)
* Allow users to supply a `commands.json` file for newer or older versions of Minecraft [#42](https://github.com/vanilla-technologies/mcfunction-debugger/issues/42)
* Freezing the `gametime` while suspended [#18](https://github.com/vanilla-technologies/mcfunction-debugger/issues/18)
* Freezing the age of all entities while suspended (this is currently only done for area_effect_clouds) [#24](https://github.com/vanilla-technologies/mcfunction-debugger/issues/24)
* Support debugging multiple datapacks at once [#9](https://github.com/vanilla-technologies/mcfunction-debugger/issues/9)
* Support debugging `load.json` and `tick.json` [#8](https://github.com/vanilla-technologies/mcfunction-debugger/issues/8)
* Support storing the `result`/`success` of a `function` command with `execute store` [#11](https://github.com/vanilla-technologies/mcfunction-debugger/issues/11)
* Setting `randomTickSpeed` to 0 while suspended [#14](https://github.com/vanilla-technologies/mcfunction-debugger/issues/14)

## Caveats

Unfortunately a program can always behave slightly differently when being debugged.
Here are some problems you might encounter with McFunction-Debugger.

### Operating on Dead Entities

In a Minecraft function you can kill an entity and then continue using it.
For example, consider the following datapack:

`example:sacrifice_pig`:
```
summon pig ~ ~ ~ {Tags: [sacrifice]}
execute as @e[type=pig,tag=sacrifice] run function example:perform_necromancy
```

`example:perform_necromancy`:
```
say I am still alive
function example:kill_me
say I am dead inside
```

`example:kill_me`:
```
kill @s
```

After the function `example:kill_me` is executed the pig is dead, yet it speaks to us from the other side.
This cannot be handled by the debugger.
If you try to debug the function `example:sacrifice_pig` it will crash:
```
[Pig] I am still alive
Selected entity was killed!
Start a new debugging session with '/function debug:<your_namespace>/<your_function>'
Executed 145 commands from function 'debug:example/sacrifice_pig'
```

### Hitting the Maximum Command Chain Length

By default Minecraft only executes up to 65536 commands per tick.
Since the debug datapack needs to run many commands in addition to the commands of your datapack, you might hit this limit when debugging a very large datapack.
You can tell by looking at how many commands where executed from the function.
When you see something like:
```
Executed 65536 commands from function 'debug:resume'
```
You should stop the debug session with `/function debug:stop` and add more breakpoints to avoid running so many commands at once or increase the command limit with:
```
/gamerule maxCommandChainLength 2147483647
```

### Chunkloading

If a chunk that contains an entity required for debugging is unloaded, while a function is suspended on a breakpoint, the debug session will crash, if you try to resume the execution.

This can for example happen if you go far away or if the function operates in a chunk that is only loaded temporarily (for instance by a `teleport` command or by going through a portal).
