#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ComptimeKind {
    Sandbox,
    Target,
    Host,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ModuleGraphRef {
    Runtime,
    Comptime(ComptimeKind),
}

impl ModuleGraphRef {
    pub fn default_comptime(self) -> Option<Self> {
        self.target_comptime()
    }

    pub fn comptime(self, kind: ComptimeKind) -> Option<Self> {
        match kind {
            ComptimeKind::Sandbox => self.sandbox_comptime(),
            ComptimeKind::Target => self.target_comptime(),
            ComptimeKind::Host => self.host_comptime(),
        }
    }

    pub fn target_comptime(self) -> Option<Self> {
        match self {
            ModuleGraphRef::Runtime => Some(ModuleGraphRef::Comptime(ComptimeKind::Target)),
            ModuleGraphRef::Comptime(comptime_kind) => None,
        }
    }

    pub fn host_comptime(self) -> Option<Self> {
        match self {
            ModuleGraphRef::Runtime => Some(ModuleGraphRef::Comptime(ComptimeKind::Host)),
            ModuleGraphRef::Comptime(comptime_kind) => None,
        }
    }

    pub fn sandbox_comptime(self) -> Option<Self> {
        match self {
            ModuleGraphRef::Runtime => Some(ModuleGraphRef::Comptime(ComptimeKind::Sandbox)),
            ModuleGraphRef::Comptime(comptime_kind) => match comptime_kind {
                ComptimeKind::Sandbox => None,
                ComptimeKind::Target | ComptimeKind::Host => {
                    Some(Self::Comptime(ComptimeKind::Sandbox))
                }
            },
        }
    }
}
