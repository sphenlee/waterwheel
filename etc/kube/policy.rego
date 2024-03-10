package waterwheel

default authorize = false

authorize {
    is_readonly
}

authorize {
    input.http.headers["x-seal-mechanism"] == "seal/formlogin"
}

is_readonly {
    input.action == "Get"
}
is_readonly {
    input.action == "List"
}
