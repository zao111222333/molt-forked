# Migrating from Molt 0.5 to 0.6

Molt 0.6 deliberately resets several Rust APIs so the interpreter can grow toward Tcl 8.6 without
preserving runtime states that are no longer useful.

## Interpreter construction

`Interp::default()` remains the simplest constructor for `Interp<()>`. Applications with a custom
context or command set now use `InterpBuilder`:

```rust
use molt_forked::prelude::*;

let commands = gen_command!(
    AppContext,
    [],
    [("about", cmd_about, "display application information")],
);

let mut interp = InterpBuilder::new(AppContext::default(), commands)
    .standard_library(StandardLibrary::Slim)
    .name("my-app")
    .build();
```

`StandardLibrary::Full` requires compiling `molt-forked` with the `full` feature. `molt-shell`
enables it by default; `molt-wasm` exposes it as its own `full` feature.

## Commands and context

The former public `Command` type is now `CommandSet`. `CommandKind`, built-in command constants and
dispatch internals are hidden implementation details. Continue declaring application commands with
`gen_command!` and `gen_subcommand!`; their tuple syntax is unchanged from 0.5.

`Interp` no longer exposes its context field. Use `context()` or `context_mut()`. Use `name()` for
the interpreter name.

## Syntax and REPL completion

`Interp::complete` and exception-based incomplete-input signaling have been removed. Use the shared
syntax API:

```rust
use molt_forked::syntax::{self, ParseStatus};

match syntax::script_status(source) {
    ParseStatus::Incomplete { .. } => { /* read another line */ }
    ParseStatus::Complete | ParseStatus::Invalid => { /* submit to the interpreter */ }
}
```

Use `syntax::analyze_script` when tokens or diagnostics are also needed. Token ranges are UTF-8 byte
ranges and cover the complete source without gaps or overlap.

## Values

`Value::as_bytes()` exposes Tcl bytearray data. With `full`, `Value::as_bignum()` and
`Value::get_bignum()` expose arbitrary-precision integers. Fixed-width callers can continue using
`as_int()`; it returns an error when a bignum does not fit.
