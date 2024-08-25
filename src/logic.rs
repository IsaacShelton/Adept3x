macro_rules! implies {
    ($x:expr, $y:expr) => {
        !($x) || ($y)
    };
}

pub(crate) use implies;
