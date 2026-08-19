# Molt Library Crates

A Molt extension crate exports command handlers. The application includes those handlers in
its single static `gen_command!` declaration:

```rust
// In the extension crate:
use molt_forked::prelude::*;

pub fn cmd_mycommand<Ctx>(_interp: &mut Interp<Ctx>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 1, 1, "")?;
    molt_ok!("extension result")
}
```

```rust
// In the application crate:
let command = gen_command!(
    AppCtx,
    [],
    [("mycommand", my_extension::cmd_mycommand, "run the extension")],
);
let mut interp = Interp::new(AppCtx::default(), command, true, "my-app");
```

This replaces the old run-time `install`/`add_command` pattern. It lets the compiler
monomorphize the context type and keeps dispatch and help generation allocation-free.
