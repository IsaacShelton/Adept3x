use slotmap::SlotMap;
use std::sync::RwLock;

slotmap::new_key_type! { pub struct CacheDbString; }

pub struct CacheDb {
    strings: RwLock<SlotMap<CacheDbString, String>>,
}

impl CacheDb {
    pub fn new() -> Self {
        Self {
            strings: Default::default(),
        }
    }

    pub fn alloc_str(&self, content: &str) -> CacheDbString {
        self.strings.write().unwrap().insert(content.into())
    }

    pub fn read_str<Ret>(&self, key: CacheDbString, mut f: impl FnMut(&str) -> Ret) -> Ret {
        f(self.strings.read().unwrap().get(key).unwrap())
    }
}
