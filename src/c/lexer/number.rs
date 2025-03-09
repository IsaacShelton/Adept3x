use super::error::LexError;
use crate::c::token::{CTokenKind, Integer, IntegerSuffix};

#[derive(Copy, Clone, Debug)]
enum Longness {
    Regular,
    Long,
    ExtraLong,
}

#[derive(Copy, Clone, Debug)]
enum Sign {
    Regular,
    Unsigned,
}

pub fn lex_number(number: String) -> Result<CTokenKind, LexError> {
    let number = remove_delimiters(number);
    let (number, radix) = lex_radix(&number);
    let (number, sign, longness) = lex_suffix(&number);

    dbg!(&number, &sign, &longness);

    use IntegerSuffix::*;

    let requested = match (sign, longness) {
        (Sign::Regular, Longness::Regular) => Int,
        (Sign::Regular, Longness::Long) => Long,
        (Sign::Regular, Longness::ExtraLong) => LongLong,
        (Sign::Unsigned, Longness::Regular) => UnsignedInt,
        (Sign::Unsigned, Longness::Long) => UnsignedLong,
        (Sign::Unsigned, Longness::ExtraLong) => UnsignedLongLong,
    };
    dbg!(&requested);

    // The correct type for an integer literal is whichever of these fits it first
    // (Section 6.4.4.1 of the C standard)
    let order: &[IntegerSuffix] = match radix {
        10 => match requested {
            Int => &[Int, Long, LongLong],
            UnsignedInt => &[UnsignedInt, UnsignedLong, UnsignedLongLong],
            Long => &[Long, LongLong],
            UnsignedLong => &[UnsignedLong, UnsignedLongLong],
            LongLong => &[LongLong],
            UnsignedLongLong => &[UnsignedLongLong],
        },
        _ => match requested {
            Int => &[
                Int,
                UnsignedInt,
                Long,
                UnsignedLong,
                LongLong,
                UnsignedLongLong,
            ],
            UnsignedInt => &[UnsignedInt, UnsignedLong, UnsignedLongLong],
            Long => &[Long, UnsignedLong, LongLong, UnsignedLongLong],
            UnsignedLong => &[UnsignedLong, UnsignedLongLong],
            LongLong => &[LongLong, UnsignedLongLong],
            UnsignedLongLong => &[UnsignedLongLong],
        },
    };

    for possible_type in order {
        if let Some(integer) = Integer::try_new(number, *possible_type, radix) {
            return Ok(CTokenKind::Integer(integer));
        }
    }

    Err(LexError::UnrepresentableInteger)
}

fn remove_delimiters(mut number: String) -> String {
    number.remove_matches('\'');
    number
}

fn lex_radix(number: &str) -> (&str, u32) {
    if number.starts_with("0") {
        // SAFETY: The is okay since we know that a codepoint starts
        // at index 1 and the characters we are checking for are represented
        // with a single byte in UTF-8

        if let Some(b'x' | b'X') = number.as_bytes().get(1) {
            return (&number[2..], 16);
        }

        if let Some(b'b' | b'B') = number.as_bytes().get(1) {
            return (&number[2..], 2);
        }

        return (number, 8);
    }

    (number, 10)
}

fn lex_suffix(number: &str) -> (&str, Sign, Longness) {
    let (number, sign) =
        unsuffix(number, &["U", "u"], Some(Sign::Unsigned)).unwrap_or((number, None));

    let (number, longness) = unsuffix(number, &["LL", "ll"], Longness::ExtraLong)
        .or_else(|| unsuffix(&number, &["L", "l"], Longness::Long))
        .unwrap_or((number, Longness::Regular));

    let (number, sign) = sign
        .map(|sign| (number, sign))
        .or_else(|| unsuffix(&number, &["U", "u"], Sign::Unsigned))
        .unwrap_or((number, Sign::Regular));

    (number, sign, longness)
}

fn unsuffix<'a, T>(number: &'a str, suffixes: &[&str], meaning: T) -> Option<(&'a str, T)> {
    for suffix in suffixes {
        if let Some(stripped) = number.strip_suffix(suffix) {
            return Some((stripped, meaning));
        }
    }
    None
}
