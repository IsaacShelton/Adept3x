#!/usr/bin/env sh

self="$(dirname -- "$0")"

function compile() {
    echo "[-] Compiling '$1'"
    ../target/debug/adept $1/main.adept
}

function expect_fail_compile() {
    echo "[-] Expecting failure to compile '$1'"
    ../target/debug/adept $1/main.adept
}

pushd "$self" > /dev/null
cargo build
compile and_or
compile annotation_groups
compile array_access
compile bitwise_operators
compile c_printf
compile comparison_operators
compile float_literal
compile function_parameters
compile function_simple
compile global_variables
compile if
compile if_elif_else
compile if_eval
compile integer_and_float_literals_combining
compile integer_hex_literals
compile integer_literal_conforming
compile integer_signed_overflow
compile integer_unsigned_overflow
compile integer_value_conforming
compile late_initialization
compile math_floats
compile math_simple
compile member_pod
compile nested_expressions
compile object_mutation
compile op_then_assign
compile reference_counted
compile return
compile return_message
compile signed_unsigned_promotion
compile structure_definitions
compile structure_literals
compile structure_pod
compile unary_operators
compile variables
compile variables_override
compile variables_typed
compile while

echo "[!] RUNNING CASES WITH EXPECTED FAILURE"

expect_fail_compile _should_fail/mismatching_yielded_types
expect_fail_compile _should_fail/partial_initialization
expect_fail_compile _should_fail/uninitialized_member
expect_fail_compile _should_fail/uninitialized_simple
popd > /dev/null

