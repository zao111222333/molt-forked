# Embedding Molt

This chapter explains how to embed Molt in a Rust application:

* creating an interpreter;
* [defining application commands](./commands.md);
* [evaluating commands and scripts](./eval.md);
* exposing a [custom shell](./shell.md).

For the standard command set and no application context, use `Default`:

```rust
use molt_forked::Interp;

let mut interp = Interp::default();
let value = interp.eval("expr {2 + 2}")?;
```

Applications that add commands declare one static command set with `gen_command!` and pass
it, together with a typed context, to `Interp::new`:

```rust
use molt_forked::prelude::*;

#[derive(Default)]
struct AppCtx {
    calls: usize,
}

fn cmd_ping(interp: &mut Interp<AppCtx>, _argv: &[Value]) -> MoltResult {
    interp.context.calls += 1;
    molt_ok!("pong")
}

let command = gen_command!(
    AppCtx,
    [],
    [("ping", cmd_ping, "reply with pong")],
);
let mut interp = Interp::new(AppCtx::default(), command, true, "my-app");
```

The generated dispatcher is a `match`, and its aligned help is a `&'static str`. Command
names and descriptions are checked at compile time; there is no run-time command registry
or help-layout allocation.
