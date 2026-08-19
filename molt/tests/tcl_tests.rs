use molt_forked::prelude::*;
#[test]
fn test_tcl_tests() {
    // FIRST, create and initialize the interpreter.
    // Set the recursion limit down from its default, or the interpreter recursion
    // limit test will fail (the Rust stack will overflow).
    type YourCtx = ();
    let mut interp = Interp::new(
        (YourCtx::default(), TestCtx::new()),
        gen_command!(
            (YourCtx, TestCtx),
            [
                (_SOURCE, cmd_source),
                (_EXIT, cmd_exit),
                (_PARSE, cmd_parse),
                (_PDUMP, cmd_pdump),
                (_PCLEAR, cmd_pclear)
            ],
            [("test", test_cmd, "run a test case")]
        ),
        true,
        "molt-test",
    );
    interp.set_recursion_limit(200);

    let args = vec![String::from("tests/all.tcl")];

    assert!(test_harness(&mut interp, &args).is_ok());
}
