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
compile math_simple
compile return
compile return_message
compile variables
compile variables_override
compile variables_typed
popd > /dev/null
