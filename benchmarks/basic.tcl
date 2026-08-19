# Initial set of benchmarks.
pclear

benchmark ok-1.1 {ok, no arguments} {
    ok
}

benchmark ok-1.2 {ok, one argument} {
    ok a
}

benchmark ok-1.3 {ok, two arguments} {
    ok a b
}

benchmark ident-1.1 {ident, simple argument} {
    ident a
}

benchmark incr-1.1 {incr a} {
    incr a
}

benchmark set-1.1 {set var value} {
    set myvar 5
}

proc benchproc {value} {
    expr {$value + 1}
}

benchmark proc-1.1 {procedure call with one argument} {
    benchproc 41
}

benchmark expr-1.1 {mixed arithmetic expression} {
    expr {(17 * 19 + 23) / 2}
}

benchmark list-1.1 {serialize a list of six items} {
    string length [list this that theother foo bar quux]
}

benchmark dict-1.1 {serialize a dictionary of six entries} {
    string length [dict create one 1 two 2 three 3 four 4 five 5 six 6]
}

benchmark join-1.1 {join a list of six items} {
    join {this that theother foo bar quux} ,
}

benchmark subcommand-1.1 {dispatch a string subcommand} {
    string length abcdefghijklmnopqrstuvwxyz
}

pdump
