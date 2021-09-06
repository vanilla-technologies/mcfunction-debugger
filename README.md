# mcfunction-debugger

mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any Minecraft mods.

## Debug your datapack in three steps

1. Add `# breakpoint` lines in your *.mcfunction files
2. Generate a debug datapack and load it in Minecraft
3. Start debugging any of your functions by executing the command `/function debug:<your_namespace>/<your_function>`

## Generating a debug datapack

Currently it is necessary to [install Rust](https://www.rust-lang.org/tools/install) and clone this reporsitory in order to generate a debug datapack.
Executables for Windows, Linux and Mac will be provided with the first release.

Build an executable for Windows with following command:

`cargo build --release`

and run it with

`mcfunction-debugger [FLAGS] [OPTIONS] --input <DATAPACK> --output <DATAPACK>`

### Flags

* `shadow`: When this is set to true the generated datapack will additionally contain functions with the same name as the functions in the input datapack.
These functions will simply forward to the appropriate function in the `debug` namespace. When using this make sure to disable the input datapack to avoid name clashes.\
\
This can be helpful when executing a function from a command block, because you don't have to change the function call to debug the function. Note however that calling a debug function inside an execute prevents the debugger to suspend the execute. For example if the command `execute as @e run function my_namespace:my_function` hits a breakpoint in my_function if there is more than one entity my_function will be called again, resulting in an error like: "Cannot start debugging my_namespace:my_function, because a function is already suspended at a breakpoint!".


### Options

* `input`: The datapack to generate a debug datapack for. This has to be a directory containing a `pack.mcmeta` file.
* `log-level`: The log level can also be configured via the environment variable `LOG_LEVEL`.
* `namespace`: The internal namespace of the generated datapack.\
Default value: `mcfd`.\
The namespace is used for all internal functions in the generated datapack and as a prefix for all scoreboard objectives and tags. By specifying a different namespace with max. 7 characters you can avoid name clashes. The generated functions in the `debug` namespace such as `debug:install` and `debug:resume` are unaffected by this option.
* `output`: The directory that should become the generated debug datapack.
On Windows this is typically a directory in the datapacks directory of your world, for example: \
`%APPDATA%\.minecraft\\saves\\Your-World\\datapacks\\debug-my-datapack`
