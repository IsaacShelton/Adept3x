mod all_symbols;
mod ast_file;
mod configure_job;
mod module;
mod name_scope;
mod namespace;
mod type_decl_ref;

pub use all_symbols::AstWorkspaceSymbols;
use arena::{Arena, ArenaMap, Idx, new_id_with_niche};
use ast::{Enum, ExprAlias, Func, Global, Impl, RawAstFile, Struct, Trait, TypeAlias};
pub use ast_file::*;
use ast_workspace_settings::{Settings, SettingsId, SettingsRef};
use configure_job::ConfigureJob;
use fs_tree::{Fs, FsNodeId};
pub use module::*;
pub use name_scope::*;
pub use namespace::*;
use source_files::SourceFiles;
use std::collections::{HashMap, VecDeque};
pub use type_decl_ref::TypeDeclRef;

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
        for (fs_node_id, module_settings) in original_module_folders.into_iter() {
            module_folders.insert(fs_node_id, settings.alloc(module_settings));
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
            funcs: self.symbols.all_funcs.get_span(name_scope.funcs).collect(),
            structs: self
                .symbols
                .all_structs
                .get_span(name_scope.structs)
                .collect(),
            enums: self.symbols.all_enums.get_span(name_scope.enums).collect(),
            globals: self
                .symbols
                .all_globals
                .get_span(name_scope.globals)
                .collect(),
            type_aliases: self
                .symbols
                .all_type_aliases
                .get_span(name_scope.type_aliases)
                .collect(),
            expr_aliases: self
                .symbols
                .all_expr_aliases
                .get_span(name_scope.expr_aliases)
                .collect(),
            traits: self
                .symbols
                .all_traits
                .get_span(name_scope.traits)
                .collect(),
            impls: self.symbols.all_impls.get_span(name_scope.impls).collect(),
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
