# Test Script: rename

test rename-1.1 {rename error} {
    rename
} -error {wrong # args: should be "rename oldName newName"}

test rename-1.2 {rename no such command} {
    rename nonesuch newname
} -error {can't rename "nonesuch": command doesn't exist}

test rename-2.1 {rename command to ""} -setup {
    proc hello {} { return "hello" }
} -body {
    rename hello ""
    hello
} -error {unknown command "hello", valid commands:
tcl:
  append, array, assert_eq, break, catch, continue, dict, error, expr, for, foreach, global, if, incr, info, join, lappend, lindex, list, llength, proc, puts, rename, return, set, string, throw, time, unset, while, source, exit, parse, pdump, pclear
molt-test:
  test  run a test case
  help  [-all]
procedure:
  doit, lexpr}

test rename-2.2 {rename command} -setup {
    proc hello {} { return "hello" }
} -body {
    rename hello howdy
    howdy
} -cleanup {
    rename howdy ""
} -ok hello
