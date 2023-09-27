This repository has moved to https://codeberg.org/vanilla-technologies/mcfunction-debugger to avoid GitHubs two factor authentication (2FA) requirement. We believe that Microsofts decision to force all code contributors to use 2FA is very problematic for the following reasons:

1. 2FA significantly increases the risk of irreversible account loss. This is very different to 2FA for something like online banking where in the worst case you can contact your bank and verify your identity to regain access. With GitHub however, if you loose your phone and backup codes (both of which is possible), you will never gain access to your account again.
2. The decision to require 2FA for every code contributor seems very needless. Yes software supply chain attacks are a thing, but not every code contributor on GitHub is responsible for widely used libraries. It's quite the opposite: most code contributors are not responsible for widely used libraries and their code is reviewed and merged by those that are. Also, the details of the 2FA requirement seem arbitrary. Why for example is email not accepted as a second factor or why can WebAuth only be a second second factor and not a primary second factor? Just to make it really hard to not use a phone for 2FA? It feels like a "trust us, we know what's good for you" attitude from Microsoft and it is scary to think what arbitrary decision could come next.
3. Depending on how you use passwords the account security is not necessary improved that much by using 2FA, especially if it is forced onto people that don't want to use it. So why is there no opt out?
4. Many other developers publicly stated that they are leaving GitHub because of this, so staying on GitHub would prevent any code contributions from these people. This makes finding good contributors even harder than before. By moving to https://codeberg.org everyone can continue to contribute to this project.
5. Unfortunately Microsoft does not allow mail as a second factor and some companies do not allow you to bring your private phone to work or install proprietary software (such authenticators) for security reasons. This means 2FA can actually completely prevent you from logging into the website in some circumstances. This is really sad, because it can make it harder for professional developers at companies that use free and open source software to return something to the community.
6. Not everyone owns/can afford a smartphone or dedicated authenticator hardware and Microsoft makes it very inconvenient to use 2FA without that by requiring you to install authenticator software on every development machine. This discourages code contributions from poor people.

2FA is a good technology, but it should be up to repository owners to decide whether it is appropriate for the project at hand. Requiring 2FA for all code contributions, even for code that is reviewed and merged by other people, is completely unnecessary and discourages contributions.

[![Minecraft: Java Edition 1.14.1 - 1.19.4](https://img.shields.io/badge/Minecraft%3A%20Java%20Edition-1.14.1%20--%201.19.4-informational)](https://www.minecraft.net/store/minecraft-java-edition)
![Minecraft: Bedrock Edition unsupported](https://img.shields.io/badge/Minecraft%3A%20Bedrock%20Edition-unsupported-critical)\
[![crates.io](https://img.shields.io/crates/v/mcfunction-debugger)](https://crates.io/crates/mcfunction-debugger)
[![Build Status](https://img.shields.io/github/actions/workflow/status/vanilla-technologies/mcfunction-debugger/ci.yml?branch=main)](https://github.com/vanilla-technologies/mcfunction-debugger/actions/workflows/ci.yml)

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
