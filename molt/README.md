# molt-forked

`molt-forked` is an embeddable Tcl interpreter for Rust. Version 0.5.0 uses static command
dispatch, compile-time validated help declarations, a safe `Value` implementation, and typed
application context.

```rust
use molt_forked::Interp;

let mut interp = Interp::default();
let four = interp.eval("expr {2 + 2}")?;
assert_eq!(four.as_int()?, 4);
```

Application commands are ordinary Rust functions collected by `gen_command!`:

```rust
use molt_forked::prelude::*;

fn cmd_square(_interp: &mut Interp<()>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 2, "integer")?;
    let integer = argv[1].as_int()?;
    molt_ok!(integer * integer)
}

let command = gen_command!(
    (),
    [],
    [("square", cmd_square, "square an integer")],
);
let mut interp = Interp::new((), command, true, "example");
assert_eq!(interp.eval("square 5")?.as_int()?, 25);
```

Ensemble commands use `gen_subcommand!`; its three-field entries need no manual padding and
automatically receive aligned `-help` output. See the
[Molt Book](https://zao111222333.github.io/molt-forked) and
[API documentation](https://docs.rs/molt-forked) for details.
