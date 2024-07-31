use std::str::FromStr;

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
    type Err = !;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut chunks = text.split('.');

        let parse_digit = |x| u8::from_str(x).ok();

        let major = chunks.next().and_then(parse_digit).unwrap_or(0);
        let minor = chunks.next().and_then(parse_digit).unwrap_or(0);
        let release = chunks.next().and_then(parse_digit).unwrap_or(0);

        Ok(Self {
            major,
            minor,
            release,
        })
    }
}
