# datatypes

### C Types:

- `bool` : equivalent to `bool` in C
- `char` : equivalent to `char` in C
- `uchar` : equivalent to `unsigned char` in C
- `schar` : equivalent to `signed char` in C
- `short` : equivalent to `short` in C
- `ushort` : equivalent to `unsigned short` in C
- `int` : equivalent to `int` in C
- `uint` : equivalent to `unsigned int` in C
- `long` : equivalent to `long` in C
- `ulong` : equivalent to `unsigned long` in C
- `longlong` : equivalent to `long long` in C
- `ulonglong` : equivalent to `unsigned long long` in C
- `float`: equivalent to `float` in C
- `double`: equivalent to `double` in C

### Specific:

- `i8` : 8-bit signed integer
- `i16` : 16-bit signed integer
- `i32` : 32-bit signed integer
- `i64` : 64-bit signed integer
- `u8` : 8-bit unsigned integer
- `u16` : 16-bit unsigned integer
- `u32` : 32-bit unsigned integer
- `u64` : 64-bit unsigned integer
- `f32` : 32-bit float
- `f64` : 64-bit float

### Pointers

See `docs/pointers.md`

### Overflow/Underflow Behavior

How overflow/underflow is handled depends on the operator used:

- `a + b` - Runtime Error
- `a +% b` - Wrapping Add, will wrap similar to in C
- `a +^ b` - Saturating Add, will be as close to the true result as possible

These also apply for subtraction and multiplication. They also have `+=`-like
variants as well.

Addition:

- `a + b`
- `a +% b`
- `a +^ b`
- `a += b`
- `a +%= b`
- `a +^= b`

Subtraction:

- `a - b`
- `a -% b`
- `a -^ b`
- `a -= b`
- `a -%= b`
- `a -^= b`

Multiplication:

- `a * b`
- `a *% b`
- `a *^ b`
- `a *= b`
- `a *%= b`
- `a *^= b`
