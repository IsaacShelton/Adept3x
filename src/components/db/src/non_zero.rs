#[macro_export]
macro_rules! non_zero {
    ($n:expr) => {{
        const _: [(); 1] = [(); if $n == 0 { panic!("Can't be zero!") } else { 1 }];
        unsafe { NonZero::new_unchecked($n) }
    }};
}
