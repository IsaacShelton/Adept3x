## Normal Pointers

#### Types

- `ptr<T>` : Pointer - (same as `T*` in C)
- `ptr_const<T>` : Pointer-to-Const - (same as `const T*` in C)

#### Address-of

- `ptr<T>` : `&my_variable` **OR** `my_variable.ptr()` **OR** `ptr(my_variable)`
- `ptr_const<T>` : `&my_variable` **OR** `my_variable.ptr_const()` **OR** `ptr_const(my_variable)`

These work on anything of type `deref<T>`.

#### Dereferencing

You can dereference a pointer with either `*my_variable `**OR** `my_variable.deref()`

See "Dereferenced Pointers" section for more details on dereferencing.

## Borrowed-Checked Pointers

- `ref<T>` - Immutable Borrow Type (like `const T*` in C, except with safety
  rules)
- `mut<T>` - Mutable Borrow Type (like `T*` in C, except with safety rules)
- `ref<..., T>` - Same as `ref<T>`, except with constraints
- `mut<..., T>` - Same as `mut<T>`, except with constraints

#### Address-of

- For `ref<T>` - `my_variable.ref()` OR `ref(my_variable)` (similar to
  `&my_variable` in C)
- For `mut<T>` - `my_variable.mut()` OR `mut(my_variable)` (similar to
  `&my_variable` in C)

#### Dereferencing

You can dereference a pointer with either `*my_variable `**OR** `my_variable.deref()`

See "Dereferenced Pointers" section for more details on dereferencing.

## Dereferenced Pointers

#### Types

- `deref<T>` : Read-Write Memory Location (same as a dereferenced pointer in C)
- `deref_const<T>` : Read-Only Memory Location (same as a dereferenced
  pointer-to-const in C)

#### Dereferencing

- `deref<T>` : `*my_variable` **OR** `my_variable.deref()` **OR** `deref(my_variable)`
- `deref_const<T>` : `*my_variable` **OR** `my_variable.deref_const()` **OR** `deref_const(my_variable)`
