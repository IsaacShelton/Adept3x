use crate::{Cache, Kv, Pf, ShouldPersist, TaskStatusKind, rt_st_in::cache::COMPILER_BUILT_AT};
use serde::ser::{SerializeMap, SerializeSeq};

impl<'e, P: Pf> serde::Serialize for Cache<'e, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut header = serializer.serialize_map(Some(3))?;
        header.serialize_entry("v", &format!("{:X}", COMPILER_BUILT_AT))?;
        header.serialize_entry("kv", &self.kv)?;
        header.end()
    }
}

impl<'e, P: Pf> serde::Serialize for Kv<'e, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut include = vec![];

        for (key, value) in self.inner.iter() {
            if !key.should_persist() {
                continue;
            }
            match value {
                Some(status) => match &status.kind {
                    TaskStatusKind::Running(_) => (),
                    TaskStatusKind::Completed(completed) => {
                        include.push((
                            key,
                            status.task.changed_at,
                            status.task.verified_at,
                            &status.task.requested,
                            &completed.aft,
                        ));
                    }
                    TaskStatusKind::Restarting(_) => (),
                },
                None => (),
            }
        }

        let mut seq = serializer.serialize_seq(Some(include.len()))?;
        for entry in include {
            seq.serialize_element(&entry)?;
        }
        seq.end()
    }
}
