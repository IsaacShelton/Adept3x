use crate::Id;

pub trait NewId: Id {}

#[macro_export]
macro_rules! new_id {
    ($name: ident, $ty: ty) => {
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name($ty);

        impl ::arena::Id for $name {
            const MAX: usize = <$ty>::MAX as usize;

            #[inline]
            fn from_usize(idx: usize) -> Self {
                Self(idx as $ty)
            }

            #[inline]
            fn into_usize(self) -> usize {
                self.0 as usize
            }

            #[inline]
            fn successor(self) -> Self {
                Self(self.0 + 1)
            }
        }

        impl ::arena::NewId for $name {}
    };
}

#[macro_export]
macro_rules! new_id_with_niche {
    ($name: ident, $ty: ty) => {
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(::core::num::NonZero<$ty>);

        impl ::arena::Id for $name {
            const MAX: usize = (<$ty>::MAX - 1) as usize;

            #[inline]
            fn from_usize(idx: usize) -> Self {
                if !cfg!(debug_assertions)
                    && const { ::core::mem::size_of::<$ty>() < ::core::mem::size_of::<u64>() }
                {
                    // If NOT in debug mode and using a type smaller than u64, manually unwrap
                    Self(::core::num::NonZero::new((idx + 1) as $ty).expect("arena id overflowed"))
                } else {
                    // Otherwise,
                    // In release, we should run out of memory before this would ever happen
                    // In debug, the overflow of the addition should panic
                    unsafe { Self(::core::num::NonZero::new_unchecked((idx + 1) as $ty)) }
                }
            }

            #[inline]
            fn into_usize(self) -> usize {
                (self.0.get() - 1) as usize
            }

            #[inline]
            fn successor(self) -> Self {
                Self(self.0.checked_add(1).expect("successor must exist"))
            }
        }

        impl ::arena::NewId for $name {}
    };
}
