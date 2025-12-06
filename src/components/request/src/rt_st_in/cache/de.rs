use crate::{
    Cache, Completed, Kv, Pf, Task, TaskStatus, TaskStatusKind, rt_st_in::cache::COMPILER_BUILT_AT,
};
use std::marker::PhantomData;

impl<'de, P: Pf> serde::Deserialize<'de> for Cache<'de, P> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(CacheDeserializeVisitor {
            _phantom_p: PhantomData,
            _phantom_de: PhantomData,
        })
    }
}

struct CacheDeserializeVisitor<'de, P: Pf> {
    _phantom_p: std::marker::PhantomData<P>,
    _phantom_de: std::marker::PhantomData<&'de ()>,
}

impl<'de, P: Pf> serde::de::Visitor<'de> for CacheDeserializeVisitor<'de, P> {
    type Value = Cache<'de, P>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a cache map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut version_hex: Option<String> = None;
        let mut kv: Option<Kv<'de, P>> = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "v" => {
                    let s: String = map.next_value()?;
                    version_hex = Some(s);
                }
                "kv" => {
                    kv = Some(map.next_value()?);
                }
                other => {
                    return Err(serde::de::Error::unknown_field(other, &["v", "kv"]));
                }
            }
        }

        let version_hex = version_hex.ok_or_else(|| serde::de::Error::missing_field("v"))?;
        let kv = kv.ok_or_else(|| serde::de::Error::missing_field("kv"))?;

        if u64::from_str_radix(&version_hex, 16).map_err(serde::de::Error::custom)?
            != COMPILER_BUILT_AT
        {
            return Err(serde::de::Error::custom(
                "Cache file is incompatible with the current compiler",
            ));
        }

        Ok(Cache { kv })
    }
}

impl<'e, P: Pf> serde::Deserialize<'e> for Kv<'e, P> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'e>,
    {
        deserializer.deserialize_seq(KvDeserializeVisitor {
            _phantom_p: PhantomData,
            _phantom_e: PhantomData,
        })
    }
}

struct KvDeserializeVisitor<'e, P: Pf> {
    _phantom_p: std::marker::PhantomData<P>,
    _phantom_e: std::marker::PhantomData<&'e ()>,
}

impl<'de, P: Pf> serde::de::Visitor<'de> for KvDeserializeVisitor<'de, P> {
    type Value = Kv<'de, P>;

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a kv array")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut kv = Kv::default();

        while let Some(item) = seq.next_element()? {
            let (key, verified_at, changed_at, requested, aft): (
                P::Req<'de>,
                P::Rev,
                P::Rev,
                Vec<P::Req<'de>>,
                P::Aft<'de>,
            ) = item;

            kv.inner.insert(
                key,
                Some(TaskStatus {
                    kind: TaskStatusKind::Completed(Completed { aft }),
                    task: Task {
                        verified_at,
                        changed_at,
                        requested,
                    },
                }),
            );
        }

        Ok(kv)
    }
}
