pub use super::Id;
use core::num::NonZero;

macro_rules! impl_id_for_nums {
    ($($ty:ty),*) => {$(
        impl Id for $ty {
            const MAX: usize = <$ty>::MAX as usize;

            #[inline]
            fn from_usize(idx: usize) -> Self {
                assert!(idx <= <Self as Id>::MAX);
                idx as $ty
            }

            #[inline]
            fn into_usize(self) -> usize {
                self as usize
            }

            #[inline]
            fn successor(self) -> Self {
                self + 1
            }
        }

        impl Id for NonZero<$ty> {
            const MAX: usize = (<$ty>::MAX - 1) as usize;

            #[inline]
            fn from_usize(idx: usize) -> Self {
                assert!(idx <= <Self as Id>::MAX);
                unsafe { NonZero::new_unchecked((idx + 1) as $ty) }
            }

            #[inline]
            fn into_usize(self) -> usize {
                (self.get() - 1) as usize
            }

            #[inline]
            fn successor(self) -> Self {
                self.checked_add(1).expect("successor must exist")
            }
        }
    )*};
}

impl_id_for_nums!(u8, u16, u32, u64, usize);
