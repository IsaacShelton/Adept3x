#!/usr/bin/env sh

self="$(dirname -- "$0")"

function compile() {
    echo "[-] Compiling '$1'"
    ../target/debug/adept $1/main.adept
}

pushd "$self" > /dev/null
cargo build
compile c_printf
compile comparison_operators
compile function_parameters
compile function_simple
compile integer_literal_conforming
compile integer_signed_overflow
compile integer_unsigned_overflow
compile integer_value_conforming
compile math_simple
compile member_pod
compile return
compile return_message
compile structure_definitions
compile structure_literals
compile structure_pod
compile variables
compile variables_override
compile variables_typed
popd > /dev/null

