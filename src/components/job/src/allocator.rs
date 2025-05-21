use std::num::NonZero;
use std_ext::BoxedSlice;
pub type BumpAllocator = bumpalo::Bump;

pub struct BumpAllocatorPool {
    pub allocators: BoxedSlice<BumpAllocator>,
}

impl BumpAllocatorPool {
    pub fn new(num_threads: NonZero<usize>) -> Self {
        Self {
            allocators: (0..num_threads.get())
                .map(|_| BumpAllocator::new())
                .collect(),
        }
    }

    pub fn len(&self) -> NonZero<usize> {
        NonZero::new(self.allocators.len()).unwrap()
    }
}
