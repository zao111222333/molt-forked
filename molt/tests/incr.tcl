# Test Suite: incr command

test incr-1.1 {incr command no args} {
    incr
} -error {wrong # args: should be "incr varName ?increment?"}

test incr-2.1 {incr new var} -body {
    incr a
} -cleanup {
    unset a
} -ok {1}

test incr-2.2 {incr existing var} -body {
    set a 5
    incr a
} -cleanup {
    unset a
} -ok {6}

test incr-2.3 {var is set} -body {
    incr a
    set a
} -cleanup {
    unset a
} -ok {1}

test incr-2.4 {increment can be specified} -body {
    set a 5
    incr a 7
    set a
} -cleanup {
    unset a
} -ok {12}

test incr-3.1 {incr scalar as array} -body {
    set x ""
    incr x(0)
} -error {can't read "x(0)": variable isn't array}

test incr-3.2 {invalid existing integer is not treated as zero} {
    set x nope
    incr x
} -error {expected integer but got "nope"}

test incr-3.3 {Tcl integer prefixes and signed minimum} {
    set output {}
    foreach value {010 0o10 0O10 0b10 0B10 0x10 0X10 -0x10 -9223372036854775808} {
        set x $value
        lappend output [incr x 0]
    }
    set output
} -ok {8 8 8 2 2 16 16 -16 -9223372036854775808}

if {$molt_full} {
    test incr-4.1 {full profile promotes arbitrary precision integers} {
        set x 9223372036854775807
        incr x
    } -ok 9223372036854775808

    test incr-4.2 {dict incr uses arbitrary precision integers} {
        set data {x 9223372036854775807}
        dict incr data x
        dict get $data x
    } -ok 9223372036854775808
}
