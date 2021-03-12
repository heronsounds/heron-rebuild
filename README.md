# heron-rebuild (`hr`) #

`heron-rebuild` is a workflow-based build system designed for complicated, branching build workflows. Its config file syntax and overall design is based on [ducttape](https://github.com/jhclark/ducttape). `ducttape` was designed for building AI systems, and `heron-rebuild` can help there, too, but it was designed with complicated software builds in mind.

## why? ##

Our audio plugins, like [pgs-1](https://heronsounds.com/products/pgs-1), are composed of a core rust library, with two different plugin formats written in C++ (VST and AudioUnit), and targeting two different operating systems and two different CPU architectures. To build just the Mac version, we need to:

- `cargo build` the rust library twice, once for `x86_64` and once for `aarch64`
- use `lipo` to combine the two rust libraries into a single universal library
- build the C++ wrapper library twice, once for each format, linking to the rust library and using the rust headers
- construct plugin bundles for each format that can be loaded into a DAW
- use `pkgbuild` to construct a `.pkg` installer for each plugin format
- use `productbuild` to construct a combined `.pkg` installer that can install both plugin formats at once

Written out as a DAG, this could look like:

```
cargo_build[x86_64]      cpp_build[vst] – pkgbuild[vst]
                   \    /                             \
                    lipo                               productbuild
                   /    \                             /
cargo_build[aarch64]     cpp_build[au]  – pkgbuild[au]
```

Each of those steps can be run in either debug or release mode, and should depend on the corresponding debug or release version of its input. And we need to run a different set of steps when building for Windows (using [cross](https://github.com/cross-rs/cross) instead of `cargo build`, for example). It's not pictured, but we also use [cbindgen](https://github.com/mozilla/cbindgen) to generate header files from our rust code for the c++ builds to depend on.

What we'd like is to define each of these steps once, and parameterize them with different values according to which branch of the DAG we're on. This is more than our poor `scripts` directory could handle, unfortunately.

## what `heron-rebuild` is ##

It's a bunch of bash snippets with dependencies that can be run as a single workflow. The snippets are called "tasks" and look like this:

```
task cargo_build
  > rustlib="target/$target/$profile/$lib_name"
  :: target=(Target: x86_64=x86_64-apple-darwin aarch64=aarch64-apple-darwin)
  :: profile=(Profile: debug release)
  :: release_flag=(Profile: debug="" release="--release")
{
  cargo build $release_flag --target $target
}
```

Those parenthesized values are *branchpoints*; they tell us that on one branch of the workflow, we'll call cargo with `--target x86_64-apple-darwin` and on another branch we'll run the same command again, except with `--target aarch64-apple-darwin`.

The `> rustlib="target/$target/$profile/$lib_name` tells us that this snippet produces an output file called `target/$target/$profile/$lib_name` which we can reference with the name `$rustlib` in other snippets.

## what it's not ##

It's not *really* a build system; it doesn't know anything about compiling code, for example, and it won't check your source files for changes to determine whether to compile them or not. It just ties together different bash commands and makes sure their dependencies are met.

It's also not a full-featured command runner; for that, we recommend [just](https://github.com/casey/just).

## using `heron-rebuild` ##

```
> hr -h
Usage: hr [OPTIONS]

Options:
  -c, --config <FILE>           Workflow definition file [env: HERON_REBUILD_CONFIG=] [default: rebuild.hr]
  -p, --plan <PLAN>             Name of target plan
  -t, --task <TASK>             Name of target task
  -x, --invalidate              Invalidate specified task
  -o, --output <DIR>            Output directory [env: HERON_REBUILD_OUTPUT=] [default: output]
  -y, --yes                     Bypass user confirmation
  -v, --verbose                 Print additional debugging info
  -b, --branch <K1.V1[+K2.V2]>  Target branch
  -B, --baseline                Use baseline branch ('-b Baseline.baseline')
  -n, --dry-run                 Dry run; print info but don't modify anything
  -h, --help                    Print help
  -V, --version                 Print version
```

A typical `heron-rebuild` call might look like this:

```
> hr -p main -c rebuild.hr
```

This tells `hr` to run the tasks defined in the *plan* called "main" in the config file called `rebuild.hr`. The `-p` option is always required when running a workflow, but the `-c` option can be omitted, in which case `hr` will look for a file called `rebuild.hr` in the current directory and use it if it exists.

Config files look like this:
```
> cat rebuild.hr
plan main {
  reach replace_text
}

task write_text > output=write_text_output.txt {
  echo "foo" > $output
}

task replace_text < input=$output@write_text > output=replace_text_output.txt {
  cat $input | sed 's/foo/bar/' > $output
}
```

Let's run the above command, with the above config file, and see what happens:

```
> hr -p main -c rebuild.hr
[command output omitted for brevity]
> tree output
├── branchpoints.txt
├── replace_text
│   ├── Baseline.baseline -> realizations/Baseline.baseline
│   └── realizations
│       └── Baseline.baseline
│           ├── exit_code
│           ├── replace_text_output.txt
│           ├── stderr.txt
│           ├── stdout.txt
│           └── task.sh
└── write_text
    ├── Baseline.baseline -> realizations/Baseline.baseline
    └── realizations
        └── Baseline.baseline
            ├── exit_code
            ├── write_text_output.txt
            ├── stderr.txt
            ├── stdout.txt
            └── task.sh
> cat output/write_text/Baseline.baseline/write_text_output.txt
foo
> cat output/replace_text/Baseline.baseline/replace_text_output.txt
bar
```

What's happened here is that `hr` has created a directory `output`, with subdirectories for the two tasks `write_text` and `replace_text`, and nested in each of those directories is a `.txt` file that was created by running the bash code from the config file.

Neither of these tasks has any branching functionality, but if they did, we'd see multiple subdirectories in `realizations` for each branch (a task is *realized* when it is run on a specific branch of the workflow).

Note that all output is written to `output` in the directory `hr` was called from by default. This can be overriden with the `-o|--output` option.

Note also that `hr` created several additional files in each task's directory:
- `exit_code`: this gets written when the task completes, so we can check if it succeeded later
- `stderr.txt` and `stdout.txt`: capture and save all output from the bash code (they are also written to the console while the task is executing)
- `task.sh`: a shell script containing exactly the commands that were run to produce this task's output (it's not actually used when executing the task, but it's there as an archive for debugging)

### Re-running `heron-rebuild` ###

Each time you call `hr`, it will check the output directory for already-completed tasks, and use their outputs without re-running them if it can. If a task's bash code fails during workflow execution, the entire workflow execution stops, but any successful tasks can still be reused. At this point, you can correct the error, call `hr` again, and finish executing the workflow without having to redo any of the earlier steps that succeeded.

If you'd like to *force* `hr` to re-run tasks that already completed successfully, see the section on **Invalidating tasks** below.

## syntax overview ##

```
# you must have at least one plan:
plan plan_name {
  reach task_name
}

# variables defined in a global block are available to all tasks:
global {
  unquoted_literal=values/can_be_unquoted
  quoted_literal="values can be in double quotes"

  interpolated="values can interpolate variables, like $unquoted_literal"

  task_output=$output@task_name
}

task task_name
  < literal_input=/home/me/some-file.txt
  > ouput=relative/to/task
  :: param=$unquoted_literal
{
  echo $unquoted_literal
  mkdir -p $(dirname $output)
  cp $literal_input $output
}
```

## in-depth syntax ##

### plans

plans consist of three parts: a *name*, a *goal task*, and a *branch*:

```
plan plan_name {
  reach goal_task via (Profile: debug) * (Os: mac)
}
```

The goal task is introduced by the `reach` keyword, and is the task that we will work backward from to create a list of tasks to execute: each task referenced in `goal_task`'s inputs, and all of its dependencies, will be executed, finally finishing the workflow after executing `goal_task`.

The branch is introduced by the `via` keyword, and is specified with cross-product notation. The branch above `(Profile: debug) * (Os: mac)` consists of two *branchpoints*, `Profile` and `Os`, with `Profile` set to `debug` and `Os` set to `mac`.

Currently, only one goal task and one branch -- one value for each branchpoint -- is allowed in a plan. Relaxing this restriction is on our roadmap, and once that's complete, you'll be able to specify e.g. `(Profile: debug) * (Os: mac windows)` to run a workflow with branches for both `mac` and `windows`. In the meantime, you might want to set up your config file with two plans, like so:

```
plan mac {
  reach goal_task via (Profile: debug) * (Os: mac)
}

plan win {
  reach goal_task via (Profile: debug) * (Os: windows)
}
```

### values

There are several types of values in a workflow:

```
# literal values without spaces can be unquoted:
unquoted_var=literal_value_with_no_spaces
unquoted_var2=/literals/can/also/be/paths

# literal values can be written in double quotes
sentence="put a whole sentence in double quotes"

# values can refer to other values:
renamed=$unquoted_var

# interpolate variables in double quotes:
interpolated="the sentence above is: $sentence"

# the path to a task output can be specified with '@'.
# this variable contains the path to the output file "output_var_name" from the task "task_name":
task_output=$output_var_name@task_name

# values can be *branched* with parentheses.
# this variable has value "brew" on branch (Os: mac), "choco" on branch (Os: windows), etc.:
pkg_mgr=(Os: mac=brew windows=choco ubuntu=apt-get)

# when we want to create a value with the same name as a branchpoint, we can omit the value name.
# this assignment is shorthand for profile=(Debug: debug=debug release=release):
profile=(Debug: debug release)

# another example of a branched value using shorthand notation:
os=(Os: mac windows ubuntu)

# a value evaluated for a specific branch (a "branch graft") can be specified with brackets.
# this variable has the value of the variable "profile" on branch (Profile: release),
# regardless of which branch is currently being evaluated:
grafted=$profile[Profile: release]

# branch grafts can be combined with task outputs:
task_output_release=$output_var_name@task_name[Profile: release]
```

### global config

Values can be specified, one per line, in a `global` block:

```
global {
  var1="hi there"
  var2=$output@some_task
}
```

These values are then usable by any task in the workflow.

### tasks

tasks are where the bulk of the logic in a workflow file lives. They look like this:

```
task cargo_build
  @cargo
  > lib="target/$profile/myrustlib"
  :: release_flag=(Profile: debug="" release="--release")
{
  cargo build $release_flag
}
```

This tells `hr` to make a task that:
- runs in the `cargo` module directory (modules are explained below)
- produces one output known to `hr`, called `lib` and located at `target/$profile/myrustlib` (which will evaluate to `target/debug/myrustlib` or `target/release/myrustlib` depending on which branch we're evaluating: `(Profile: debug)` or `(Profile: release)`).
- takes a parameter `release_flag`, which is defined differently depending on whether we're on `(Profile: debug)` or `(Profile: release)`.
- Runs the command `cargo build $release_flag`.

Everything before the opening brace is called the *task header*. Task headers consist of a name and an optional list of values to define how the task should run and its relationship to other tasks. These values can be any of:

- any number of input files, specified with `<` (not shown above).
- any number of output files, specified with `>`.
- any number of parameters, specified with `::`.
- zero or one module definitions, specified with `@`.

The variable names of any inputs, outputs, and parameters defined in the header are available to the code block below (see how we use `$release_flag` in the code above).

These values (with the exception of modules), use the same value syntax above, so for example:

```
< input=(Branched: branch1=val1 branch2=val2)
> output1=$config_var_defined_in_a_global_block
> output2="put/$variable_interpolation/in/double-quotes"
:: param1=$grafted_variable[Branchpoint: branch1]
```

Multiple task values of a single type can be defined in a space-separated list:

```
< input1=foo input2=bar
```

And multiple of these lists can be defined on a single line:

```
< input1=foo input2=bar > output=output :: param1=x param2=y
```

Additionally, there are some shorthands available in task values that are not available in global blocks. Any input or output value defined without an equal sign:

```
< input
> output
```

is interpreted the same as:

```
< input=input
> output=output
```

This is useful if you e.g. don't care what the name of your output file is, just that it exists.

Parameters can be specified with an `@` sign, like:

```
param=@
```

This tells `workflow` to look for a variable named `param` defined in a global block, and use its value. There are a few other shortcuts that we won't get to here, too. See the examples dir for more info.

#### inputs (`<`)

Inputs to a task are files, and the main thing that differentiates them from other task values is that they are checked for existence before the task is run. If any of a task's defined input files doesn't exist immediately before the task runs, execution stops. `workflow` doesn't care if they're files or directories, just that they exist.

As with other values, they can be branched or grafted.

#### outputs (`>`)

Outputs of a task are files, and what differentiates them from other task values is that they are checked for existence *after* the task is run. If any of a task's defined output files doesn't exist immediately after the task runs, the task is considered to have failed, and execution stops. `workflow` doesn't care if outputs are files or directories, just that they exist. And, as with other values, they can be branched or grafted.

Outputs should be defined as relative paths, relative to the directory where the task code will run (more on task directories later).

#### params (`::`)

Params are not checked for existence at any point. They can be defined as literal strings, or references to config values defined elsewhere, but not as task outputs.

#### modules (`@`)

Modules are just a single identifier preceded by an `@` sign, like `@cargo`. In order for a task header like `task cargo_build @cargo` to work, there must be a module `cargo` defined elsewhere in the config file, like:

```
module cargo=/home/me/code/my-crate
```

Normally, for tasks without a `@module` specified, `workflow` will create a new directory for each execution of the task and run its code there. All of the outputs are assumed to be relative to this new directory, and other tasks that depend on them will expect to find them there.

For a `@module` task, `workflow` will execute the code in the module directory instead, and then copy output files from the module directory back into the task directory (where other tasks can find them).

This is mainly useful for build commands, which rely on source code that exists in a specific location and that we don't necessarily want to have to copy into a new directory each time we run the workflow. See `examples` for examples.

## Invalidating tasks ##

The `-x` flag tells `hr` to invalidate a task that has already been run:

```
> hr -x -t pkgbuild -b Framework=vst
```

The above will invalidate the task called `pkgbuild` for the branch `(Framework: vst)`. This means that the next time the workflow is run, it will re-run that task, regardless of whether it succeeded or not. All of that task's dependents will also re-run, since their inputs are now considered invalid.

The `-t` flag specifying a task name is always required, but the `-b` flag specifying a branch is optional. If it's omitted, `hr` will invalidate *all* realizations of the task on all branches.

Multiple branchpoints can be specified together for more complicated branches, either with multiple `-b` flags:

```
> hr -x -t pkgbuild -b Framework=vst -b Profile=release
```

Or by chaining them together with `+`:

```
> hr -x -t pkgbuild -b Framework=vst+Profile=release
```

## Roadmap

Getting most of the following done should get us to a 1.0 release:

- More complex plans: multiple goal nodes, multiple branches
- Increase test coverage
- More thorough error messages
- Allow imports in config files so they can be split across multiple files
  - Global settings file in user's home directory
- Allow built-in variables in config, like `$HOME`
- Allow overriding config values on the command line
- Allow users to validate workflow from the command line without executing it
- Allow users to inspect workflows with command-line options
- Expand this documentation
- Decide if `hr` is really a good name...

After that, we can look at expanding the basic functionality:

- Add options for executing code remotely or in a container
- Use multiple threads to execute tasks simultaneously
- Introduce ways to reuse common code across tasks
- Interact with terminal multiplexers?
- Add some more expressive syntax e.g. branches defined as ranges (N: 0..10)
