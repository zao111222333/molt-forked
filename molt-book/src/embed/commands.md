# Defining Commands

A Rust command is a function that receives a typed interpreter and the complete Molt argument
list, including the command name. It returns `MoltResult`.

```rust
use molt_forked::prelude::*;

fn cmd_ident(_interp: &mut Interp<()>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 2, "value")?;
    molt_ok!(argv[1].clone())
}
```

Application commands are installed as `(name, handler, help)` triples in `gen_command!`:

```rust
let command = gen_command!(
    (),
    [],
    [("ident", cmd_ident, "return a value unchanged")],
);
let mut interp = Interp::new((), command, true, "example");
```

Names and help descriptions must be string literals. The macro rejects duplicate names and
the reserved `help` command, preserves declaration order, and formats Unicode-aware help at
compile time. The native-command list can expose optional built-ins such as `source`:

```rust
let command = gen_command!(
    (),
    [(_SOURCE, cmd_source)],
    [("ident", cmd_ident, "return a value unchanged")],
);
```

## Typed application context

Commands that manipulate application data use the interpreter's generic context directly.
No context ID, hash table, or downcast is needed.

```rust
use molt_forked::prelude::*;

#[derive(Default)]
struct AppCtx {
    text: String,
}

fn cmd_append_text(interp: &mut Interp<AppCtx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 2, "value")?;
    interp.context.text.push_str(argv[1].as_str());
    molt_ok!()
}

let command = gen_command!(
    AppCtx,
    [],
    [("append_text", cmd_append_text, "append to application text")],
);
let mut interp = Interp::new(AppCtx::default(), command, true, "text-app");
```

Several commands can share the same context type and access different fields. Rust performs
the field access statically, so this abstraction adds no run-time type lookup.

## Ensemble commands

An ensemble is a command with subcommands. `gen_subcommand!` generates its static dispatcher
and automatically aligned `-help` output:

```rust
use molt_forked::prelude::*;

fn cmd_browser_alert(_interp: &mut Interp<()>, _argv: &[Value]) -> MoltResult {
    molt_ok!()
}

fn cmd_browser_confirm(_interp: &mut Interp<()>, _argv: &[Value]) -> MoltResult {
    molt_ok!(true)
}

fn cmd_browser(interp: &mut Interp<()>, argv: &[Value]) -> MoltResult {
    let dispatch = gen_subcommand!(
        (),
        1,
        [
            ("-alert", cmd_browser_alert, "show an alert"),
            ("-confirm", cmd_browser_confirm, "show a confirmation"),
        ],
    );
    dispatch(interp, argv)
}
```

The second argument is the index of the subcommand in `argv`; use a larger index for a nested
ensemble. `-help` is reserved and supplied automatically.

Applications can model object commands by passing an object identifier to a static ensemble
and storing the objects in `interp.context`. Object data can still be created and destroyed at
run time while the command dispatcher remains static.

## Molt procedures

Molt procedures are written in Tcl and defined with `proc`. A crate can embed a script with
`include_str!` and evaluate it during interpreter initialization:

```rust
let mut interp = Interp::default();
interp
    .eval(include_str!("commands.tcl"))
    .expect("could not load commands.tcl");
```
