package waterwheel

default allow = false

is_readonly {
    input.action = "Get"
}
is_readonly {
    input.action = "List"
}

allow {
    is_readonly
}

allow {
    input.http["x-waterwheel-user"] = "admin"
}

