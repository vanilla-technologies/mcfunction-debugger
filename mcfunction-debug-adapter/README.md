# McFunction-Debug-Adapter

This directory contains the [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/) implementation of McFunction-Debugger.

McFunction-Debug-Adapter only supports the **single session mode** with communication via _stdin_ and _stdout_. To start executing an mcfunction file the development tool needs to send a `launch` request (the `attach` request is **not** supported).

## Execution Context

The debugged function will be executed with a `schedule` command, so it runs without an `@s` entity at the world's origin position.

## Launch Arguments

In order for the debug adapter to connect to Minecraft it needs a few arguments as part of the `launch` request:

### program

Path to the mcfunction file to debug. The mcfunction file must be contained in a datapack with a `pack.mcmeta` file.

### minecraftWorldDir

The directory containing the Minecraft world the debug adapter should connect to.

For single player this is typically a directory within the saves directory:
* Windows: `%appdata%\.minecraft\saves\`
* GNU/Linux: `~/.minecraft/saves/`
* Mac: `~/Library/Application Support/minecraft/saves/`

For servers it is specified in `server.properties`.

### minecraftLogFile

The path to Minecraft's log file.

For single player this is typically at these locations:
* Windows: `%appdata%\.minecraft\logs\latest.log`
* GNU/Linux: `~/.minecraft/logs/latest.log`
* Mac: `~/Library/Application Support/minecraft/logs/latest.log`

For servers it is at `logs/latest.log` in the server directory.

### Example
```json
{
  "program": "C:/Users/Herobrine/my_datapack/data/my_namespace/functions/main.mcfunction",
  "minecraftWorldDir": "C:/Users/Herobrine/AppData/Roaming/.minecraft/saves/New World",
  "minecraftLogFile": "C:/Users/Herobrine/AppData/Roaming/.minecraft/logs/latest.log"
}
```

## Command Line Interface

`mcfunction-debug-adapter [FLAGS] [OPTIONS] --input <DATAPACK> --output <DATAPACK>`

### Flags

#### --help

Prints help information.

#### --version

Prints version information.

### Options

#### --log-file

Path to a log file. If specified the debug adapter will create this file on startup and write log messages to it.

#### --log-level

The log level can also be configured via the environment variable `LOG_LEVEL`. Defaults to `INFO`.
