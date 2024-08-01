use std::{fmt::Display, str::FromStr};

const ADEPT_VERSION: AdeptVersion = AdeptVersion {
    major: 3,
    minor: 0,
    release: 0,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdeptVersion {
    major: u8,
    minor: u8,
    release: u8,
}

impl Display for AdeptVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.release == 0 {
            write!(f, "{}.{}", self.major, self.minor)
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor, self.release)
        }
    }
}

impl PartialOrd for AdeptVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AdeptVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.release.cmp(&other.release))
    }
}

impl FromStr for AdeptVersion {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut chunks = text.split('.');
        let parse_digit = |x| u8::from_str(x).map_err(|_| ());
        let major = chunks.next().map(parse_digit).transpose()?.unwrap_or(0);
        let minor = chunks.next().map(parse_digit).transpose()?.unwrap_or(0);
        let release = chunks.next().map(parse_digit).transpose()?.unwrap_or(0);

        Ok(Self {
            major,
            minor,
            release,
        })
    }
}
