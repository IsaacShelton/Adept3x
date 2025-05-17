/*
    ==================  support/optional_string/src/lib.rs  ===================
    A small library for accepting optional string parameters that sucks less.

    Optimized for the case of passing NoneStr, &str, String, Option<&str>,
    and Option<String>.

    Only caveat is that dynamically computed values must be wrapped
    in `LazyString`, which isn't needed very often.
    ---------------------------------------------------------------------------
*/

pub trait OptionalString {
    fn to_option_string(self) -> Option<String>;
}

pub struct NoneStr;
pub struct LazyString<T: ToString>(pub T);

impl OptionalString for &str {
    fn to_option_string(self) -> Option<String> {
        Some(self.to_string())
    }
}

impl OptionalString for String {
    fn to_option_string(self) -> Option<String> {
        Some(self.to_string())
    }
}

impl OptionalString for Option<&str> {
    fn to_option_string(self) -> Option<String> {
        self.map(|value| value.to_string())
    }
}

impl OptionalString for Option<String> {
    fn to_option_string(self) -> Option<String> {
        self
    }
}

impl OptionalString for NoneStr {
    fn to_option_string(self) -> Option<String> {
        None
    }
}

impl<T: ToString> OptionalString for LazyString<T> {
    fn to_option_string(self) -> Option<String> {
        Some(self.0.to_string())
    }
}
