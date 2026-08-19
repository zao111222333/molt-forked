# Custom Shells

A custom Molt shell creates an interpreter with its application command set, then passes it to
`molt_shell::repl` for interactive use or `molt_shell::script` for a file.

```rust
use molt_forked::prelude::*;
use std::env;

fn cmd_hello(_interp: &mut Interp<()>, argv: &[Value]) -> MoltResult {
    check_args(1, argv, 2, 2, "name")?;
    println!("Hello, {}", argv[1].as_str());
    molt_ok!()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = gen_command!(
        (),
        [(_SOURCE, cmd_source)],
        [("hello", cmd_hello, "greet someone")],
    );
    let mut interp = Interp::new((), command, true, "hello-shell");

    if args.len() > 1 {
        molt_shell::script(&mut interp, &args[1..]);
    } else {
        molt_shell::repl(&mut interp);
    }
}
```

The REPL prompt can be customized by setting the `tcl_prompt1` Molt variable to a script that
returns the desired prompt.
