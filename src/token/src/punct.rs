use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

#[derive(Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Punct([u8; 4]);

impl Punct {
    pub const fn new(s: &'static str) -> Self {
        let str_bytes = s.as_bytes();
        let mut chars: [u8; 4] = *b"\0\0\0\0";
        let mut i = 0;

        while i < str_bytes.len() && i < 4 {
            let c = str_bytes[i];

            if c >= 128 {
                break;
            }

            chars[i] = c;
            i += 1;
        }

        Self(chars)
    }

    pub fn len(&self) -> usize {
        self.0.iter().position(|c| *c == b'\0').unwrap_or(4)
    }

    #[inline]
    pub const fn is(&self, possible: &'static str) -> bool {
        self.const_eq(Punct::new(possible))
    }

    #[inline]
    pub const fn const_eq(&self, other: Punct) -> bool {
        u32::from_ne_bytes(self.0) == u32::from_ne_bytes(other.0)
    }

    #[inline]
    pub const fn is_any(&self, possible: &[&'static str]) -> bool {
        let mut i = 0;

        while i < possible.len() {
            if self.const_eq(Punct::new(possible[i])) {
                return true;
            }
            i += 1;
        }

        false
    }

    pub fn as_str(&self) -> &str {
        str::from_utf8(&self.0[..self.len()]).unwrap()
    }
}

impl Display for Punct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Debug for Punct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Punct").field(&self.as_str()).finish()
    }
}
