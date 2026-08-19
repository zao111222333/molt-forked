# Test Script: rename

test rename-1.1 {rename error} {
    rename
} -error {wrong # args: should be "rename oldName newName"}

test rename-1.2 {rename no such command} {
    rename nonesuch newname
} -error {can't rename "nonesuch": command doesn't exist}

if {$molt_full} {
    test rename-2.1 {rename command to ""} -setup {
        proc hello {} { return "hello" }
    } -body {
        rename hello ""
        hello
    } -error {unknown command "hello", valid commands:
builtins:
  append, apply, array, assert_eq, break, catch, continue, concat, dict, error, eval, expr, for, foreach, global, if, incr, info, join, lappend, lassign, lindex, linsert, list, llength, lmap, lrange, lrepeat, lreplace, lreverse, proc, puts, rename, return, set, split, string, subst, switch, throw, time, try, unset, uplevel, upvar, while, source, exit, parse
molt-test:
  test  run a test case
  help  [-all]
procedure:
  doit, lexpr}
} else {
    test rename-2.1 {rename command to ""} -setup {
        proc hello {} { return "hello" }
    } -body {
        rename hello ""
        hello
    } -error {unknown command "hello", valid commands:
builtins:
  append, array, assert_eq, break, catch, continue, dict, error, expr, for, foreach, global, if, incr, info, join, lappend, lindex, list, llength, proc, puts, rename, return, set, string, throw, time, unset, while, source, exit, parse
molt-test:
  test  run a test case
  help  [-all]
procedure:
  doit, lexpr}
}

test rename-2.2 {rename command} -setup {
    proc hello {} { return "hello" }
} -body {
    rename hello howdy
    howdy
} -cleanup {
    rename howdy ""
} -ok hello
