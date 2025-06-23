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
use std::collections::{HashMap, HashSet, VecDeque};
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

// File <----> FS Node ID
// Module <----> Module FS Node ID
// File -----> Module
// Source Symbol -----> Namespace Scope Ref of File

// Each file has a root NamespaceScopeRef (possibly nested)
// Each file has a parent module
// Each file has an FS Node ID
// Each module has an FS Node ID

pub struct NewFile {}

impl<'source_files> AstWorkspace<'source_files> {
    pub fn new(
        fs: Fs,
        raw_files: HashMap<FsNodeId, RawAstFile>,
        source_files: &'source_files SourceFiles,
        original_module_folders: HashMap<FsNodeId, Settings>,
    ) -> Self {
        let mut settings = Arena::new();
        let default_settings = settings.alloc(Settings::default());

        let mut symbols = AstWorkspaceSymbols::default();

        // For old ASG resolution system
        let mut module_folders = ArenaMap::new();
        for (fs_node_id, module_settings) in original_module_folders.into_iter() {
            module_folders.insert(fs_node_id, settings.alloc(module_settings));
        }

        let attention = raw_files.keys().copied().collect();
        let (module_for_file, files_for_module) = compute_modules(&fs, &attention, &module_folders);

        let mut files = ArenaMap::new();

        for (fs_node_id, raw_file) in raw_files {
            let settings = module_folders
                .get(*module_for_file.get(&fs_node_id).unwrap())
                .unwrap();
            let name_scope_ref = symbols.new_name_scope(raw_file, None, *settings);

            files.insert(
                fs_node_id,
                AstFile {
                    settings: *settings,
                    names: name_scope_ref,
                },
            );
        }

        let mut all_modules = Arena::new();
        for (_, fs_node_ids) in files_for_module {
            let mut vec = Vec::new();

            for fs_node_id in fs_node_ids {
                vec.push(files.get(fs_node_id).unwrap().clone());
            }

            all_modules.alloc(Module { files: vec });
        }

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
            settings: &self.settings[file.settings],
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
    attention: &HashSet<FsNodeId>,
    module_folders: &ArenaMap<FsNodeId, SettingsRef>,
) -> (
    HashMap<FsNodeId, FsNodeId>,
    HashMap<FsNodeId, Vec<FsNodeId>>,
) {
    let mut module_for_file = HashMap::new();
    let mut files_for_module = HashMap::<FsNodeId, Vec<_>>::new();

    struct Pair {
        cursor: FsNodeId,
        module: Option<FsNodeId>,
    }

    let mut jobs = VecDeque::new();
    jobs.push_back(Pair {
        cursor: Fs::ROOT,
        module: None,
    });

    while let Some(job) = jobs.pop_front() {
        let module = if module_folders.contains_key(job.cursor) {
            Some(job.cursor)
        } else {
            job.module
        };

        if attention.contains(&job.cursor) {
            let module_fs_node_id = job.module.expect("file to be in module");
            module_for_file.insert(job.cursor, module_fs_node_id);

            files_for_module
                .entry(module_fs_node_id)
                .or_default()
                .push(job.cursor);
        }

        // SAFETY: `read_only_view` will never deadlock here because we promise
        // to not insert any children while viewing it on this same thread
        jobs.extend(
            fs.get(job.cursor)
                .children
                .read_only_view()
                .iter()
                .map(|(_, value)| value)
                .copied()
                .map(|child_fs_node_id| Pair {
                    cursor: child_fs_node_id,
                    module,
                }),
        );
    }

    (module_for_file, files_for_module)
}
