use molt_forked::prelude::*;
use molt_shell::{cmd_ident, cmd_ok, measure_cmd, BenchCtx};
use std::env;

fn main() {
    // FIRST, get the command line arguments.
    let args: Vec<String> = env::args().collect();
    type YourCtx = ();

    // NEXT, if there's at least one then it's a subcommand.
    if args.len() > 1 {
        let subcmd: &str = &args[1];

        match subcmd {
            "bench" => {
                let mut interp = Interp::new(
                    (YourCtx::default(), BenchCtx::new()),
                    gen_command!(
                        (YourCtx, BenchCtx),
                        [
                            (_SOURCE, cmd_source),
                            (_EXIT, cmd_exit),
                            (_PARSE, cmd_parse),
                            (_PDUMP, cmd_pdump),
                            (_PCLEAR, cmd_pclear)
                        ],
                        [
                            ("ident", cmd_ident, "return a value"),
                            ("measure", measure_cmd, "record a benchmark measurement"),
                            ("ok", cmd_ok, "return an empty value")
                        ]
                    ),
                    true,
                    "molt-bench",
                );
                molt_shell::benchmark(&mut interp, &args[2..]);
            }
            "shell" => {
                let mut interp = Interp::default();
                if args.len() == 2 {
                    println!("Molt {}", env!("CARGO_PKG_VERSION"));
                    molt_shell::repl(&mut interp);
                } else {
                    molt_shell::script(&mut interp, &args[2..]);
                }
            }
            "test" => {
                let mut interp = Interp::new(
                    (YourCtx::default(), TestCtx::new()),
                    gen_command!(
                        (YourCtx, TestCtx),
                        [
                            (_SOURCE, cmd_source),
                            (_EXIT, cmd_exit),
                            (_PARSE, cmd_parse),
                            (_PDUMP, cmd_pdump),
                            (_PCLEAR, cmd_pclear),
                        ],
                        [("test", test_cmd, "run a test case")]
                    ),
                    true,
                    "molt-test",
                );
                // Keep the recursive-procedure regression below the native main-thread stack
                // limit, as the Rust integration harness does.
                interp.set_recursion_limit(200);
                if test_harness(&mut interp, &args[2..]).is_ok() {
                    std::process::exit(0);
                } else {
                    std::process::exit(1);
                }
            }
            "help" => {
                print_help();
            }
            _ => {
                eprintln!("unknown subcommand: \"{}\"", subcmd);
            }
        }
    } else {
        print_help();
    }
}

fn print_help() {
    println!("Molt {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Usage: molt <subcommand> [args...]");
    println!();
    println!("Subcommands:");
    println!();
    println!("  help                          -- This help");
    println!("  shell [<script>] [args...]    -- The Molt shell");
    println!("  test  [<script>] [args...]    -- The Molt test harness");
    println!("  bench [<script>] [args...]    -- The Molt benchmark tool");
    println!();
    println!("See the Molt Book for details.");
}
