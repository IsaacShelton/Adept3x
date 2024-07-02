#!/usr/bin/env sh

self="$(dirname -- "$0")"

function compile_module() {
    echo "[ ] Compiling '$1'"
    ../target/debug/adept $1
}

function compile() {
    echo "[=] Compiling '$1'"
    ../target/debug/adept $1/main.adept
}

function expect_fail_compile() {
    echo "[=] Expecting failure to compile '$1'"
    ../target/debug/adept $1/main.adept
}

pushd "$self" > /dev/null
cargo build
compile aliases
compile and_or
compile annotation_groups
compile array_access
compile bitwise_operators
compile c_printf
compile comparison_operators
compile defines
compile enums
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
compile math_floats
compile math_simple
compile member_pod
compile_module modules_simple
compile_module modules_headers
compile nested_expressions
compile object_mutation
compile op_then_assign
compile_module preprocessor_toggle
compile reference_counted
compile return
compile return_message
compile signed_unsigned_promotion
compile structure_definitions
compile structure_literals
compile structure_literals_abbr
compile structure_pod
compile unary_operators
compile variables
compile variables_override
compile variables_typed
compile while
compile zeroed

echo "[!] RUNNING CASES WITH EXPECTED FAILURE"

expect_fail_compile _should_fail/mismatching_yielded_types
expect_fail_compile _should_fail/partial_initialization
expect_fail_compile _should_fail/recursive_type_alias
expect_fail_compile _should_fail/uninitialized_member
expect_fail_compile _should_fail/uninitialized_simple
popd > /dev/null

