mod configure_job;

use arena::{Arena, ArenaMap, Idx, IdxSpan, new_id_with_niche};
use ast::{
    Enum, ExprAlias, Func, Global, Impl, NamespaceItems, RawAstFile, Struct, Trait, TypeAlias,
};
use ast_workspace_settings::{Settings, SettingsId, SettingsRef};
use attributes::Privacy;
use configure_job::ConfigureJob;
use fs_tree::{Fs, FsNodeId};
use source_files::SourceFiles;
use std::collections::{HashMap, VecDeque};

new_id_with_niche!(FuncId, u64);
new_id_with_niche!(StructId, u64);
new_id_with_niche!(EnumId, u64);
new_id_with_niche!(GlobalId, u64);
new_id_with_niche!(TypeAliasId, u64);
new_id_with_niche!(ExprAliasId, u64);
new_id_with_niche!(TraitId, u64);
new_id_with_niche!(ImplId, u64);
new_id_with_niche!(NameScopeId, u64);
new_id_with_niche!(ModuleId, u64);
new_id_with_niche!(NamespaceId, u64);

pub type FuncRef = Idx<FuncId, Func>;
pub type StructRef = Idx<StructId, Struct>;
pub type EnumRef = Idx<EnumId, Enum>;
pub type GlobalRef = Idx<GlobalId, Global>;
pub type TypeAliasRef = Idx<TypeAliasId, TypeAlias>;
pub type ExprAliasRef = Idx<ExprAliasId, ExprAlias>;
pub type TraitRef = Idx<TraitId, Trait>;
pub type ImplRef = Idx<ImplId, Impl>;
pub type NameScopeRef = Idx<NameScopeId, NameScope>;
pub type ModuleRef = Idx<ModuleId, Module>;
pub type NamespaceRef = Idx<NamespaceId, Namespace>;

#[derive(Clone, Debug)]
pub struct Namespace {
    pub name: String,
    pub names: NameScopeRef,
    pub privacy: Privacy,
}

#[derive(Clone, Debug)]
pub struct NameScope {
    pub funcs: IdxSpan<FuncId, Func>,
    pub structs: IdxSpan<StructId, Struct>,
    pub enums: IdxSpan<EnumId, Enum>,
    pub globals: IdxSpan<GlobalId, Global>,
    pub type_aliases: IdxSpan<TypeAliasId, TypeAlias>,
    pub expr_aliases: IdxSpan<ExprAliasId, ExprAlias>,
    pub traits: IdxSpan<TraitId, Trait>,
    pub impls: IdxSpan<ImplId, Impl>,
    pub namespaces: IdxSpan<NamespaceId, Namespace>,
    pub parent: Option<NameScopeRef>,
}

#[derive(Clone, Debug)]
pub struct AstFile {
    pub settings: Option<SettingsRef>,
    pub names: NameScopeRef,
}

#[derive(Debug)]
pub struct Module {
    pub settings: Option<SettingsRef>,
    pub files: Vec<AstFile>,
}

impl Module {
    pub fn funcs(&self, symbols: &AstWorkspaceSymbols) -> impl Iterator<Item = FuncRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].funcs.iter())
    }

    pub fn structs(&self, symbols: &AstWorkspaceSymbols) -> impl Iterator<Item = StructRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].structs.iter())
    }

    pub fn enums(&self, symbols: &AstWorkspaceSymbols) -> impl Iterator<Item = EnumRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].enums.iter())
    }

    pub fn globals(&self, symbols: &AstWorkspaceSymbols) -> impl Iterator<Item = GlobalRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].globals.iter())
    }

    pub fn type_aliases(
        &self,
        symbols: &AstWorkspaceSymbols,
    ) -> impl Iterator<Item = TypeAliasRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].type_aliases.iter())
    }

    pub fn expr_aliases(
        &self,
        symbols: &AstWorkspaceSymbols,
    ) -> impl Iterator<Item = ExprAliasRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].expr_aliases.iter())
    }

    pub fn traits(&self, symbols: &AstWorkspaceSymbols) -> impl Iterator<Item = TraitRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].traits.iter())
    }

    pub fn impls(&self, symbols: &AstWorkspaceSymbols) -> impl Iterator<Item = ImplRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].impls.iter())
    }

    pub fn namespaces<'a>(
        &'a self,
        symbols: &'a AstWorkspaceSymbols,
    ) -> impl Iterator<Item = NamespaceRef> {
        self.files
            .iter()
            .flat_map(|file| symbols.all_name_scopes[file.names].namespaces.iter())
    }
}

#[derive(Debug)]
pub struct AstFileView<'workspace> {
    pub settings: Option<&'workspace Settings>,
    pub funcs: &'workspace [Func],
    pub structs: &'workspace [Struct],
    pub enums: &'workspace [Enum],
    pub globals: &'workspace [Global],
    pub type_aliases: &'workspace [TypeAlias],
    pub expr_aliases: &'workspace [ExprAlias],
    pub traits: &'workspace [Trait],
    pub impls: &'workspace [Impl],
}

#[derive(Debug)]
pub struct AstWorkspace<'source_files> {
    pub source_files: &'source_files SourceFiles,
    pub fs: Fs,
    pub module_folders: ArenaMap<FsNodeId, Idx<SettingsId, Settings>>,
    pub files: ArenaMap<FsNodeId, AstFile>,
    pub settings: Arena<SettingsId, Settings>,
    pub default_settings: Idx<SettingsId, Settings>,
    pub symbols: AstWorkspaceSymbols,
    pub modules: Arena<ModuleId, Module>,
}

#[derive(Debug, Default)]
pub struct AstWorkspaceSymbols {
    pub all_funcs: Arena<FuncId, Func>,
    pub all_structs: Arena<StructId, Struct>,
    pub all_enums: Arena<EnumId, Enum>,
    pub all_globals: Arena<GlobalId, Global>,
    pub all_type_aliases: Arena<TypeAliasId, TypeAlias>,
    pub all_expr_aliases: Arena<ExprAliasId, ExprAlias>,
    pub all_traits: Arena<TraitId, Trait>,
    pub all_impls: Arena<ImplId, Impl>,
    pub all_namespaces: Arena<NamespaceId, Namespace>,
    pub all_name_scopes: Arena<NameScopeId, NameScope>,
}

impl AstWorkspaceSymbols {
    pub fn new_name_scope(
        &mut self,
        items: NamespaceItems,
        parent: Option<NameScopeRef>,
    ) -> NameScopeRef {
        let funcs = self.all_funcs.alloc_many(items.funcs);
        let structs = self.all_structs.alloc_many(items.structs);
        let enums = self.all_enums.alloc_many(items.enums);
        let globals = self.all_globals.alloc_many(items.globals);
        let type_aliases = self.all_type_aliases.alloc_many(items.type_aliases);
        let expr_aliases = self.all_expr_aliases.alloc_many(items.expr_aliases);
        let traits = self.all_traits.alloc_many(items.traits);
        let impls = self.all_impls.alloc_many(items.impls);

        let new_name_scope = self.all_name_scopes.alloc(NameScope {
            funcs,
            structs,
            enums,
            globals,
            type_aliases,
            expr_aliases,
            traits,
            impls,
            namespaces: IdxSpan::default(),
            parent,
        });

        let mut namespaces = Vec::with_capacity(items.namespaces.len());
        for namespace in items.namespaces {
            namespaces.push(Namespace {
                name: namespace.name,
                names: self.new_name_scope(namespace.items, Some(new_name_scope)),
                privacy: namespace.privacy,
            });
        }

        self.all_name_scopes[new_name_scope].namespaces =
            self.all_namespaces.alloc_many(namespaces.into_iter());
        new_name_scope
    }
}

impl<'source_files> AstWorkspace<'source_files> {
    pub fn new(
        fs: Fs,
        raw_files: HashMap<FsNodeId, RawAstFile>,
        source_files: &'source_files SourceFiles,
        original_module_folders: HashMap<FsNodeId, Settings>,
    ) -> Self {
        let mut settings = Arena::new();
        let default_settings = settings.alloc(Settings::default());

        let mut files = ArenaMap::new();
        let mut symbols = AstWorkspaceSymbols::default();

        for (fs_node_id, raw_file) in raw_files {
            let name_scope_ref = symbols.new_name_scope(raw_file, None);

            files.insert(
                fs_node_id,
                AstFile {
                    settings: None,
                    names: name_scope_ref,
                },
            );
        }

        // For old ASG resolution system
        let mut module_folders = ArenaMap::new();
        for (fs_node_id, module) in original_module_folders.into_iter() {
            module_folders.insert(fs_node_id, settings.alloc(module));
        }

        // For new ASG resolution job system
        let all_modules = compute_modules(&fs, &mut files, default_settings, &module_folders);

        let mut workspace = Self {
            fs,
            files,
            source_files,
            settings,
            default_settings,
            module_folders,
            modules: all_modules,
            symbols,
        };
        workspace.configure();
        workspace
    }

    pub fn view(&self, file: &AstFile) -> AstFileView {
        let name_scope = &self.symbols.all_name_scopes[file.names];
        AstFileView {
            settings: file.settings.map(|id| &self.settings[id]),
            funcs: &self.symbols.all_funcs[name_scope.funcs],
            structs: &self.symbols.all_structs[name_scope.structs],
            enums: &self.symbols.all_enums[name_scope.enums],
            globals: &self.symbols.all_globals[name_scope.globals],
            type_aliases: &self.symbols.all_type_aliases[name_scope.type_aliases],
            expr_aliases: &self.symbols.all_expr_aliases[name_scope.expr_aliases],
            traits: &self.symbols.all_traits[name_scope.traits],
            impls: &self.symbols.all_impls[name_scope.impls],
        }
    }

    pub fn get_owning_module(&self, fs_node_id: FsNodeId) -> Option<FsNodeId> {
        let mut fs_node_id = fs_node_id;

        loop {
            if self.module_folders.contains_key(fs_node_id) {
                return Some(fs_node_id);
            }

            if let Some(parent) = self.fs.get(fs_node_id).parent {
                fs_node_id = parent;
            } else {
                break;
            }
        }

        None
    }

    pub fn get_owning_module_or_self(&self, fs_node_id: FsNodeId) -> FsNodeId {
        self.get_owning_module(fs_node_id).unwrap_or(fs_node_id)
    }

    fn configure(&mut self) {
        let mut jobs = VecDeque::new();
        jobs.push_back(ConfigureJob::new(Fs::ROOT, self.default_settings));

        while let Some(job) = jobs.pop_front() {
            let fs_node_id = job.fs_node_id;

            let settings = self
                .module_folders
                .get(fs_node_id)
                .copied()
                .unwrap_or(job.settings);

            if let Some(ast_file) = self.files.get_mut(fs_node_id) {
                ast_file.settings = Some(settings);
            }

            // SAFETY: `read_only_view` will never deadlock here because we promise
            // to not insert any children while viewing it on this same thread
            jobs.extend(
                self.fs
                    .get(fs_node_id)
                    .children
                    .read_only_view()
                    .iter()
                    .map(|(_, value)| value)
                    .copied()
                    .map(|child_fs_node_id| ConfigureJob::new(child_fs_node_id, settings)),
            );
        }
    }
}

fn compute_modules(
    fs: &Fs,
    files: &mut ArenaMap<FsNodeId, AstFile>,
    default_settings: SettingsRef,
    module_folders: &ArenaMap<FsNodeId, SettingsRef>,
) -> Arena<ModuleId, Module> {
    let mut jobs = VecDeque::new();
    jobs.push_back(ComputeModuleJob::new(Fs::ROOT, None, default_settings));

    let mut modules = Arena::new();

    while let Some(job) = jobs.pop_front() {
        let fs_node_id = job.fs_node_id;
        let mut module_ref = job.module_ref;

        let settings = module_folders
            .get(fs_node_id)
            .copied()
            .unwrap_or(job.settings);

        if module_folders.contains_key(fs_node_id) {
            module_ref = Some(modules.alloc(Module {
                settings: Some(settings),
                files: vec![],
            }));
        }

        if let Some(ast_file) = files.get_mut(fs_node_id) {
            ast_file.settings = Some(settings);

            let Some(module_ref) = module_ref else {
                panic!(
                    "internal compiler error: This file is somehow not in a module - {}",
                    fs.get(job.fs_node_id).filename.to_string_lossy()
                );
            };

            modules[module_ref].files.push(ast_file.clone());
        }

        // SAFETY: `read_only_view` will never deadlock here because we promise
        // to not insert any children while viewing it on this same thread
        jobs.extend(
            fs.get(fs_node_id)
                .children
                .read_only_view()
                .iter()
                .map(|(_, value)| value)
                .copied()
                .map(|child_fs_node_id| {
                    ComputeModuleJob::new(child_fs_node_id, module_ref, settings)
                }),
        );
    }

    modules
}

pub struct ComputeModuleJob {
    pub fs_node_id: FsNodeId,
    pub module_ref: Option<ModuleRef>,
    pub settings: Idx<SettingsId, Settings>,
}

impl ComputeModuleJob {
    pub fn new(
        fs_node_id: FsNodeId,
        module_ref: Option<ModuleRef>,
        settings: Idx<SettingsId, Settings>,
    ) -> Self {
        Self {
            fs_node_id,
            module_ref,
            settings,
        }
    }
}
