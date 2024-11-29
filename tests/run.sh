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

function expect_fail_compile_module() {
    echo "[ ] Expecting failure to compile '$1'"
    ../target/debug/adept $1
}

pushd "$self" > /dev/null
cargo build
compile and_or
compile annotation_groups
compile array_access
compile bitwise_operators
compile c_printf
compile character_literals
compile comparison_operators
compile defines
compile enums
compile float_literal
compile function_parameters
compile function_simple
compile generics
compile global_variables
compile hello_world
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
compile member
compile_module modules_headers
compile_module modules_simple
compile multiline_comments
compile nested_expressions
compile object_mutation
compile op_then_assign
compile_module preprocessor_toggle
compile pointers
compile return
compile return_message
compile signed_unsigned_promotion
compile structure_definitions
compile structure_literals
compile structure_literals_abbr
compile unary_operators
compile type_aliases
compile ufcs
compile variables
compile variables_override
compile variables_typed
compile while
compile zeroed

echo "[!] RUNNING CASES WITH EXPECTED FAILURE"

expect_fail_compile _should_fail/mismatching_yielded_types
expect_fail_compile_module _should_fail/pragma_adept_first
expect_fail_compile _should_fail/recursive_type_alias
popd > /dev/null

