# Molt-forked

[![Crates.io](https://img.shields.io/crates/v/molt-forked.svg)](https://crates.io/crates/molt-forked)

This is a forked version of `molt`, a embeddable TCL interpreter for Rust applications. The original repository is no longer actively maintained, and this version aims to continue its development, fix bugs, and add new features.

## New in Molt-forked 0.5.0

Molt 0.5.0 adds compile-time validation and automatic Unicode-aware help layout to
`gen_subcommand!` and `gen_command!`. Command declarations now use unpadded
`(name, handler, help)` triples. The release also removes all core `unsafe`, replaces invalid
expression and scope states with enums and safe lookups, fixes evaluation-state leaks and
custom result codes, and reduces allocations in common interpreter paths.

All workspace packages are versioned together at `0.5.0`; the WASM packages use their local
workspace dependencies.

## New in Molt-forked 0.4.1

* The subcommands now is static, we can use `gen_subcommand!` macro to init SubCommand.

## New in Molt-forked 0.4.0

* WASM runtime support, see demo at [here](https://zao111222333.github.io/molt-forked/demo/), the demo project is in `molt-wasm` and you can find that the size of compiled WASM binary is only ~600k.
* Remove `ContextMap` and related attributes / function parameters in Interpreter. Now the definiton of Interpreter is `Interp<Ctx>` (with user-defined generic `Ctx`), we can access Interpreter's Context directly via `interp.context`.
* The native commands now is static, we need to use `gen_command!` macro to init Command.
  
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

# Molt: More Or Less Tcl

**Molt-forked 0.5.0** is a minimal implementation of the TCL language for embedding in Rust apps
and for scripting Rust libraries.  Molt is intended to be:

*   **Small in size.** Embedding Molt shouldn't greatly increase the size of the
    application.

*   **Small in language.** [Standard TCL](http://tcl-lang.org) has many features
    intended for building entire software systems.  Molt is intentionally
    limited to those needed for embedding.

*   **Small in dependencies.** Including the Molt interpreter in your project shouldn't
    drag in anything else--unless you ask for it.

*   **Easy to build.** Building Standard TCL is non-trivial.  Embedding
    Molt should be as simple as using any other crate.

*   **Easy to embed.** Extending Molt with TCL commands that wrap Rust APIs should
    be easy and simple.

Hence, perfect compatibility with Standard TCL is explicitly not a goal.  Many
features will not be implemented at all (e.g., octal literals); and others may
be implemented somewhat differently where a clearly better alternative exists
(e.g., `-nocomplain` will always be the normal behavior).  In addition, Molt will
prefer Rust standards where appropriate.

On the other hand, Molt is meant to be TCL (more or less), not simply a
"Tcl-like language", so gratuitous differences are to be avoided.  One of the
goals of this document is to carefully delineate:

*   The features that have not yet been implemented.
*   The features that likely will never be implemented.
*   Any small differences in behavior.
*   And especially, any features that have intentionally been implemented in
    a different way.

## What Molt Is For

Using Molt, you can:

*   Create a shell interpreter for scripting and interactive testing of your Rust crates.
*   Provide your Rust applications with an interactive REPL for debugging and
    administration.
*   Extend your Rust application with scripts provided at compile-time or at run-time.
*   Allow your users to script your applications and libraries.

See the [`molt-sample` repo](https://github.com/wduquette/molt-sample) for a sample Molt client
skeleton.

## New in Molt 0.5.0

Version 0.5.0 adds compile-time command validation and automatic Unicode-aware help layout,
removes unsafe code from the core crate, repairs interpreter state leaks, and reduces
allocations in common evaluation paths. See the [Annotated Change Log](changes.md) for details.

## Coming Attractions

At this point Molt is capable and robust enough for real work, though the Rust-level API is
not yet completely stable.  Standard Rust `0.y.z` semantic versioning applies: ".y" changes
can break the Rust-level API, ".z" changes will not.

*   More TCL Commands
*   Testing improvements
*   Documentation improvements
*   Feature: Regex and Glob pattern matching by Molt commands
