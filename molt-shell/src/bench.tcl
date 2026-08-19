# Benchmark Library

# benchmark name description body ?count?
#
# Measures a benchmark, executing the body 10000 times by default.
proc benchmark {name description body {count 10000}} {
    measure $name $description [lindex [time $body $count] 0]
}
