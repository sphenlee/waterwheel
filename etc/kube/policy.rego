package waterwheel

default authorize = false

authorize if {
    is_readonly
}

authorize if {
    input.http.headers["x-seal-mechanism"] == "seal/formlogin"
}

is_readonly if {
    input.action == "Get"
}
is_readonly if {
    input.action == "List"
}
