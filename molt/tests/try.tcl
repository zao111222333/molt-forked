# Test Script: try

test try-1.1 {successful body is returned unchanged} {
    try {expr {2 + 3}}
} -ok 5

test try-1.2 {on error receives value and options} {
    try {error boom} on error {message options} {
        list $message [dict get $options -code]
    }
} -ok {boom 1}

test try-1.3 {on ok can transform a successful result} {
    try {expr {2 + 3}} on ok value {expr {$value * 2}}
} -ok 10

test try-1.4 {trap matches an error-code prefix} {
    try {throw {APP NETWORK TIMEOUT} boom} trap {APP NETWORK} {message options} {
        set message
    }
} -ok boom

test try-1.5 {finally runs while preserving the prior result} {
    set marker before
    set value [try {expr {6 * 7}} finally {set marker after}]
    list $value $marker
} -ok {42 after}

test try-1.6 {an exceptional finally overrides the prior result} {
    try {set value ok} finally {error cleanup}
} -error cleanup
