# Molt-forked

[![Crates.io](https://img.shields.io/crates/v/molt-forked.svg)](https://crates.io/crates/molt-forked)

This is a forked version of `molt`, a embeddable TCL interpreter for Rust applications. The original repository is no longer actively maintained, and this version aims to continue its development, fix bugs, and add new features. WASM runtime support. See [demo](https://zao111222333.github.io/molt-forked/demo/).

## TODO

+ [ ] Full tcl 8.6.14 support
+ [ ] Update molt-app to latest rustyline version

## New in Molt-forked 0.5.0

* Added the `molt-macro` compile-time helper. `gen_subcommand!` and `gen_command!` now
  validate literal names, reject collisions, and generate Unicode-width-aligned static help.
* Simplified the command declaration syntax to `(name, handler, help)` triples; the old
  manually padded four-field syntax is intentionally unsupported.
* Removed all `unsafe` from the core crate and tightened `Value`, scope, procedure, and
  expression state so invalid combinations cannot be represented.
* Reduced allocations in empty results, variable updates, list/dictionary serialization,
  `join`, and expression dispatch. Fixed recursion/scope leaks and custom `catch` codes.
* All workspace crates now use version `0.5.0` and local path dependencies.
* Release microbenchmarks and the reproducible 0.4.5 comparison are recorded in
  [`benchmarks/RESULTS.md`](benchmarks/RESULTS.md); every measured median improved.

```rust,ignore
gen_subcommand!(
    AppCtx,
    1,
    [
        ("-alert", cmd_browser_alert, "show an alert"),
        ("-confirm", cmd_browser_confirm, "show a confirmation"),
    ],
);

gen_command!(
    AppCtx,
    [(_SOURCE, cmd_source)],
    [
        ("about", cmd_about, "display app information"),
        ("browser", cmd_browser, "call browser APIs"),
    ],
);
```

## New in Molt-forked 0.4.1

* The subcommands now is static, we can use `gen_subcommand!` macro to init SubCommand.

## New in Molt-forked 0.4.0

* WASM runtime support. See [demo](https://zao111222333.github.io/molt-forked/demo/) and you can find that the size of compiled WASM binary is only ~600k.
* Remove `ContextMap` and related attributes / function parameters in Interpreter. Now the definiton of Interpreter is `Interp<Ctx>` (with user-defined generic `Ctx`), we can access Interpreter's Context directly via `interp.context`.
* The native commands now is static, we can use `gen_command!` macro to init Command.
  
  The benefit is that `molt-fork` can use `match` block to implement token matching, rather than `HashMap`
* New document ([The Molt Book](https://zao111222333.github.io/molt-forked), Code Description) is not unimplemented yet.

### Benchmark Result

+ **Command**: `cd molt-app && cargo run --release bench ../benchmarks/basic.tcl`

+ **Platform**: Intel Xeon 6348 CPU

| molt-forked `0.4.0` (time unit in Nanos) | molt `0.3.2` | Speedup (×) | Benchmark                        |
| ---------------------------------------- | ------------ | ----------- | -------------------------------- |
| 89                                       | 208          | 2.34        | ok-1.1 ok, no arguments          |
| 90                                       | 207          | 2.3         | ok-1.2 ok, one argument          |
| 97                                       | 219          | 2.26        | ok-1.3 ok, two arguments         |
| 119                                      | 209          | 1.76        | ident-1.1 ident, simple argument |
| 209                                      | 402          | 1.92        | incr-1.1 incr a                  |
| 158                                      | 311          | 1.97        | set-1.1 set var value            |
| 201                                      | 348          | 1.73        | list-1.1 list of six items       |

---

===================== Below is Origin Document =====================

---

# Molt -- More Or Less TCL
![MoltLogo.png](MoltLogo.png)

Molt is a minimal implementation of the TCL language for embedding in Rust apps and for
scripting Rust libraries.  See [The Molt Book](https://zao111222333.github.io/molt-forked) for details
and user documentation, and [docs.rs/molt](https://docs.rs/molt-forked) for the Rust API.

## Mazegen: An Example

See [Mazegen](https://github.com/wduquette/mazegen) as an example of using Molt to provide
a TCL interface for real Rust APIs.  Mazegen is a collection of maze generation code and
related infrastructure, with a Molt interface as an aid for debugging and
experimentation.  Among other things, Mazegen includes:

* A simple binding to (a small part of) the
  [**rand::thread_rng**](https://crates.io/crates/rand) crate
* The smallest beginning of a binding to the [**image**](https://crates.io/crates/image) crate
  (more to come)

## New in Molt 0.3.1

* `molt_throw!` macro
* New `Exception` methods
* Access to environment variables via the env() variable
* Additional TCL commands

See the
[Annotated Change Log](https://wduquette.github.io/molt/changes.md) in the Molt Book for
the complete list of new features by version.

## Coming Attractions

At this point Molt is capable and robust enough for real work, though the Rust-level API is
not yet completely stable.  Standard Rust `0.y.z` semantic versioning applies: ".y" changes
can break the Rust-level API, ".z" changes will not.

*   Additional TCL commands
*   Testing improvements
*   Documentation improvements
*   Feature: Regex and Glob pattern matching by Molt commands

## Why Molt Exists

Using Molt, you can:

*   Create a shell interpreter for scripting and interactive testing of your Rust crates.
*   Provide your Rust applications with an interactive REPL for debugging and
    administration.
*   Extend your Rust application with scripts provided at compile-time or at run-time.
*   Allow your users to script your applications and libraries.

See the [`molt-sample` repo](https://github.com/wduquette/molt-sample) for a sample Molt client
skeleton.

## Molt and Standard TCL

Molt is intended to be lightweight and require minimal dependencies, so that it can be added
to any project without greatly increasing its footprint.  (At present, the core
language is a single library create with no dependencies at all!)  As such, it does not provide
all of the features of Standard TCL (e.g., TCL 8.6).

At the same time, Molt's implementation of TCL should be consistent with TCL 8.6 so far as it
goes.  Some archaic commands and command features are omitted; some changes
are made so Molt works better in the Rust ecosystem. (E.g., Molt's notion of whitespace is
the same as Rust's.) All liens against Standard TCL are documented in
the [The Molt Book](https://zao111222333.github.io/molt-forked).

No effort has been made to make the Rust-level API for extending Molt in Rust look like
Standard TCL's C API; rather, the goal is to make the Rust-level API as simple and ergonomic
as possible. **Note**: A big part of this effort is defining and refining the Rust API used
to interact with and extend the interpreter. If you have comments or suggestions for
improvement, please contact me or write an issue!

## Building and Installation

The easiest approach is to get the latest Molt through `crates.io`.  Look for the
`molt`, `molt-shell`, and `molt-app` crates, or add them to your dependencies list
in `cargo.toml`.

To build Molt:

*   Install the latest stable version of Rust (1.38.0 at time of writing)
*   Clone this repository
*   To build:

```
$ cd .../molt
$ cargo build
```

* To run the interactive shell

```
$ cargo run shell
```

* To run just the language test suite

```
$ cargo run test molt/tests/all.tcl
```

Since Molt 0.2.0 the language tests are also run by `cargo test`; however, it's much easier to
see the output of the individual tests using the above command.

## Dependencies

At present, the only dependency required by the Molt core is
[indexmap](https://docs.rs/indexmap/1.3.0/indexmap/).

## Acknowledgements

I've gotten help from many people in this endeavor; here's a (necessarily partial) list.

* Mary Duquette, for the Molt logo
* Jonathan Castello, for general Rust info
* Kevin Kenny, for help with TCL numerics and general encouragement
* Don Porter, for help with TCL parsing
* rfdonnelly, for the crates.io badge, etc.
* Coleman McFarland, for improvements to `molt_shell::repl`.
* dbohdan, for TCL command implementations and advice.
* Various folks from users.rust-lang.org who have answered my questions:
    * Krishna Sannasi, for help getting `Value` to work with arbitrary user data types
    * Yandros, for pointing me at `OnceCell` and `UnsafeCell`.
    * jethrogb, for help on how to use `Ref::map` to return a `Ref<T>` of a component deep within
      a `RefCell<S>` from a function.  (Mind you, once I got it working and gave it a try I
      tore it out again, because of `BorrowMutError` panics.  But I had to try it.)
