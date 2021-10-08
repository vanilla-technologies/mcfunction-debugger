
# Command line interface

## Usage

`mcfunction-debugger [FLAGS] [OPTIONS] --input <DATAPACK> --output <DATAPACK>`

## Flags

### --help

Prints help information.

### --shadow

When this is set to true the generated datapack will additionally contain functions with the same name as the functions in the input datapack.
These functions will simply forward to the appropriate function in the `debug` namespace. When using this make sure to disable the input datapack to avoid name clashes.

This can be helpful when executing a function from a command block, because you don't have to change the function call to debug the function.
Note however that calling a debug function inside an execute command prevents the debugger from suspending the execute command.
For example, if the command `execute as @e run function my_namespace:my_function` hits a breakpoint in `my_function` and there is more than one entity, `my_function` will be called again, resulting in an error like: "Cannot start debugging my_namespace:my_function, because a function is already suspended at a breakpoint!".

### --version

Prints version information.

## Options

### --input

The datapack to generate a debug datapack for. This has to be a directory containing a `pack.mcmeta` file, for example:
```
%APPDATA%\.minecraft\saves\Your-World\datapacks\my-datapack
```

### --log-level

The log level can also be configured via the environment variable `LOG_LEVEL`.

### --namespace

The internal namespace of the generated datapack.

Default value: `mcfd`.

The namespace is used for all internal functions in the generated datapack and as a prefix for all scoreboard objectives and tags.
By specifying a different namespace with max. 7 characters you can avoid name clashes.
The generated functions in the `debug` namespace such as `debug:resume` and `debug:uninstall` are unaffected by this option.

### --output

The directory that should become the generated debug datapack.
On Windows this is typically a directory in the datapacks directory of your world, for example:
```
%APPDATA%\.minecraft\saves\Your-World\datapacks\debug-my-datapack
```
