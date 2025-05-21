mod hash_map_ext;
mod index_map_ext;

pub use hash_map_ext::*;
pub use index_map_ext::*;
use smallvec::SmallVec;

pub type BoxedSlice<T> = Box<[T]>;
pub type SmallVec1<T> = SmallVec<[T; 1]>;
pub type SmallVec2<T> = SmallVec<[T; 2]>;
pub type SmallVec4<T> = SmallVec<[T; 4]>;
pub type SmallVec8<T> = SmallVec<[T; 8]>;
pub type SmallVec16<T> = SmallVec<[T; 16]>;
