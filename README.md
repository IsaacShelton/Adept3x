# Adept3x

<img src="https://raw.github.com/IsaacShelton/Adept3x/master/.github/README_logo.png" width="240" height="240">

A language that maximizes safety and developer productivity

Work-in-progress compiler that will become Adept 3.x

```
func main {
    println("Hello World")
}
```

### Thread Safety

<img src="https://raw.github.com/IsaacShelton/Adept3x/master/.github/thread-safety-dance.gif" width="120" height="120">

Adept distinguishes between thread-safe and non-thread-safe values, and secures code against concurrency bugs at compile-time.

### Secure Type System


<img src="https://raw.github.com/IsaacShelton/Adept3x/master/.github/sync-unsync-ref.gif" width="120" height="120">

Adept distinguishes between thread-safe values (`T`) and non-thread-safe values (`unsync<T>`).

Functions can choose to accept these values either by "shared reference" (`T`/`unsync<T>`) which can extend the lifetime of the passed value, or "inert reference" (`&T`) which cannot change the lifetime but accepts  both `T` and `unsync<T>`.
