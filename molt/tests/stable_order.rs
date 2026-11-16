//! Arrays, scope variables and procedures list their names in a stable order: insertion order,
//! with `unset` and `rename ... ""` moving the last entry into the hole. Every interpreter gets
//! the same order, so a script lists the same names in every run.

use molt_forked::Interp;

fn eval(script: &str) -> String {
    let mut interp = Interp::default();
    interp.eval(script).unwrap().as_str().to_owned()
}

const ARRAY: &str = "
    foreach k {k0 k1 k2 k3 k4 k5 k6 k7 k8 k9} { set a($k) v$k }
    unset a(k2)
    list [array names a] [array get a]
";

#[test]
fn array_names_and_get_keep_insertion_order_after_an_unset() {
    assert_eq!(
        eval(ARRAY),
        "{k0 k1 k9 k3 k4 k5 k6 k7 k8} \
         {k0 vk0 k1 vk1 k9 vk9 k3 vk3 k4 vk4 k5 vk5 k6 vk6 k7 vk7 k8 vk8}"
    );
}

#[test]
fn local_variables_keep_insertion_order_after_an_unset() {
    let script = "
        proc f {} {
            foreach k {v0 v1 v2 v3 v4 v5 v6 v7 v8 v9} { set $k 1 }
            unset v2
            info vars
        }
        f
    ";
    assert_eq!(eval(script), "k v0 v1 v9 v3 v4 v5 v6 v7 v8");
}

#[test]
fn procedures_keep_insertion_order_after_a_delete() {
    let script = "
        proc p1 {} {}; proc p2 {} {}; proc p3 {} {}; proc p4 {} {}
        rename p2 {}
        info procs
    ";
    assert_eq!(eval(script), "p1 p4 p3");
}

#[test]
fn every_interpreter_lists_the_same_order() {
    // With a hash map seeded per map, separate interpreters list 50 names in different orders.
    let script = "
        for {set i 0} {$i < 50} {incr i} { set a(n$i) $i }
        array names a
    ";
    let first = eval(script);
    for _ in 0..20 {
        assert_eq!(eval(script), first);
    }
}
