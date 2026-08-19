# Tcl 8.6 list/evaluation compatibility regressions.

test concat-1.1 {concatenates lists and canonicalizes whitespace} {
    concat {a  b} {{c d}} {} e
} -ok {a b {c d} e}

test eval-1.1 {evaluates one script without altering it} {
    set value old
    eval {set value {new value}}
} -ok {new value}

test eval-1.2 {concatenates multiple arguments as lists} {
    eval {list a} {{b c}} d
} -ok {a {b c} d}

test lassign-1.1 {assigns elements and returns the remainder} {
    set remainder [lassign {a b c} first second]
    list $first $second $remainder
} -ok {a b c}

test lassign-1.2 {fills missing values with empty strings} {
    lassign {a} first second
    list $first $second
} -ok {a {}}

test lindex-8.6.1 {accepts end-relative arithmetic indices} {
    list [lindex {a b c d} end-1] [lindex {a b c d} 1+1] [lindex {a b c} end+1]
} -ok {c c {}}

test linsert-1.1 {clamps insertion indices and treats end as append} {
    list [linsert {a b} -9 x] [linsert {a b} end x] [linsert {a b} 99 x]
} -ok {{x a b} {a b x} {a b x}}

test lrange-1.1 {clamps range endpoints} {
    list [lrange {a b c} -1 1] [lrange {a b c} 2 1] [lrange {a b c} 1 end]
} -ok {{a b} {} {b c}}

test lrepeat-1.1 {repeats all value arguments} {
    lrepeat 2 a b
} -ok {a b a b}

test lreplace-1.1 {replaces and inserts ranges} {
    list [lreplace {a b c} 1 1 x] [lreplace {a b c} 2 1 x]
} -ok {{a x c} {a b x c}}

test lreverse-1.1 {reverses a list} {
    lreverse {a {b c} d}
} -ok {d {b c} a}

test split-1.1 {splits on any delimiter and preserves empty fields} {
    split {a,b;} {,;}
} -ok {a b {}}

test split-1.2 {an empty delimiter splits Unicode characters} {
    split {A值🙂} {}
} -ok {A 值 🙂}
