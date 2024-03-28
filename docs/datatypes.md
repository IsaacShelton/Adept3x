
# datatypes

### Common:
- `bool` boolean
- `int` 64-bit signed integer with overflow checks
- `uint` 64-bit unsigned integer with overflow checks
- `float` 64-bit float

### Specific:
- `i8` wrapping 8-bit signed integer
- `i16` wrapping 16-bit signed integer
- `i32` wrapping 32-bit signed integer
- `i64` wrapping 64-bit signed integer
- `u8` wrapping 8-bit unsigned integer
- `u16` wrapping 16-bit unsigned integer
- `u32` wrapping 32-bit unsigned integer
- `u64` wrapping 64-bit unsigned integer
- `f32` 32-bit float
- `f64` 64-bit float

### Low-Level:
- `pod<T>` plain-old-data T, (raw struct type without any memory management)
- `ptr<T>` pointer to type T, (pointer to raw type without any memory management)
    - **Note:** `ptr<pod<T>>` is the same as `ptr<T>`, as `ptr<T>` implies the pointed value is plain-old-data

### Other Types:
- All other types are automatically managed values
	- The compiler may choose to destruct these before the end of the scope (if no longer needed)
	- Managed values are guaranteed to be destructed when/before the last reference to them is officially lost
	- There is no guarantee that managed values that are lost at the same time will be destructed in any particular order
	- Sharing managed values between threads is safe, but modifications to the managed value should be guarded with a mutex to prevent data races
	- For managed types that require not being destructed before the end of the scope, `#[elder]` can be used to keep them alive longer.
		- e.g. `#[elder] struct MutexGuard (/* implementation details */)`
		- e.g. `#[elder] struct ShadowFileLock (/* implementation details */)`
	- Managed values are guaranteed to be destructed in the same thread that held the last reference to them

