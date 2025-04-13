#![no_std]

/*
    =======================  support/arena/src/lib.rs  ========================
    An arena library based off of indexed_arena with some improvements
    ---------------------------------------------------------------------------
*/

mod arena;
mod id;
mod idx;
mod idx_span;
mod impl_id;
mod iter;
mod lock_free_arena;
mod map;
mod map_idx;
mod map_idx_span;
mod new_id;
mod simple_type_name;
mod values;

extern crate alloc;

pub use arena::Arena;
pub use id::Id;
pub use idx::Idx;
pub use idx_span::IdxSpan;
pub use lock_free_arena::LockFreeArena;
pub use map::ArenaMap;
pub use map_idx::MapIdx;
pub use map_idx_span::MapIdxSpan;
pub use new_id::NewId;
