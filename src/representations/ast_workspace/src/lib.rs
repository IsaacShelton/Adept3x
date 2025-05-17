mod configure_job;

use arena::{Arena, ArenaMap, Idx, IdxSpan, new_id_with_niche};
use ast::{Enum, ExprAlias, Func, Global, Impl, Namespace, RawAstFile, Struct, Trait, TypeAlias};
use ast_workspace_settings::{Settings, SettingsId, SettingsRef};
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
new_id_with_niche!(NamespaceId, u64);

pub type FuncRef = Idx<FuncId, Func>;
pub type StructRef = Idx<StructId, Struct>;
pub type EnumRef = Idx<EnumId, Enum>;
pub type GlobalRef = Idx<GlobalId, Global>;
pub type TypeAliasRef = Idx<TypeAliasId, TypeAlias>;
pub type ExprAliasRef = Idx<ExprAliasId, ExprAlias>;
pub type TraitRef = Idx<TraitId, Trait>;
pub type ImplRef = Idx<ImplId, Impl>;
pub type NamespaceRef = Idx<NamespaceId, Namespace>;

#[derive(Clone, Debug)]
pub struct AstFile {
    pub settings: Option<SettingsRef>,
    pub funcs: IdxSpan<FuncId, Func>,
    pub structs: IdxSpan<StructId, Struct>,
    pub enums: IdxSpan<EnumId, Enum>,
    pub globals: IdxSpan<GlobalId, Global>,
    pub type_aliases: IdxSpan<TypeAliasId, TypeAlias>,
    pub expr_aliases: IdxSpan<ExprAliasId, ExprAlias>,
    pub traits: IdxSpan<TraitId, Trait>,
    pub impls: IdxSpan<ImplId, Impl>,
    pub namespaces: IdxSpan<NamespaceId, Namespace>,
}

#[derive(Debug)]
pub struct Module {
    pub settings: Option<SettingsRef>,
    pub files: Vec<AstFile>,
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
    pub all_funcs: Arena<FuncId, Func>,
    pub all_structs: Arena<StructId, Struct>,
    pub all_enums: Arena<EnumId, Enum>,
    pub all_globals: Arena<GlobalId, Global>,
    pub all_type_aliases: Arena<TypeAliasId, TypeAlias>,
    pub all_expr_aliases: Arena<ExprAliasId, ExprAlias>,
    pub all_traits: Arena<TraitId, Trait>,
    pub all_impls: Arena<ImplId, Impl>,
    pub all_namespaces: Arena<NamespaceId, Namespace>,
    pub all_modules: Vec<Module>,
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
        let mut all_funcs = Arena::new();
        let mut all_structs = Arena::new();
        let mut all_enums = Arena::new();
        let mut all_globals = Arena::new();
        let mut all_type_aliases = Arena::new();
        let mut all_expr_aliases = Arena::new();
        let mut all_traits = Arena::new();
        let mut all_impls = Arena::new();
        let mut all_namespaces = Arena::new();

        for (fs_node_id, raw_file) in raw_files {
            let funcs = all_funcs.alloc_many(raw_file.funcs);
            let structs = all_structs.alloc_many(raw_file.structs);
            let enums = all_enums.alloc_many(raw_file.enums);
            let globals = all_globals.alloc_many(raw_file.globals);
            let type_aliases = all_type_aliases.alloc_many(raw_file.type_aliases);
            let expr_aliases = all_expr_aliases.alloc_many(raw_file.expr_aliases);
            let traits = all_traits.alloc_many(raw_file.traits);
            let impls = all_impls.alloc_many(raw_file.impls);
            let namespaces = all_namespaces.alloc_many(raw_file.namespaces);

            files.insert(
                fs_node_id,
                AstFile {
                    settings: None,
                    funcs,
                    structs,
                    enums,
                    globals,
                    type_aliases,
                    expr_aliases,
                    traits,
                    impls,
                    namespaces,
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
            all_funcs,
            all_structs,
            all_enums,
            all_globals,
            all_type_aliases,
            all_expr_aliases,
            all_traits,
            all_impls,
            all_namespaces,
            files,
            source_files,
            settings,
            default_settings,
            module_folders,
            all_modules,
        };
        workspace.configure();
        workspace
    }

    pub fn view(&self, file: &AstFile) -> AstFileView {
        AstFileView {
            settings: file.settings.map(|id| &self.settings[id]),
            funcs: &self.all_funcs[file.funcs],
            structs: &self.all_structs[file.structs],
            enums: &self.all_enums[file.enums],
            globals: &self.all_globals[file.globals],
            type_aliases: &self.all_type_aliases[file.type_aliases],
            expr_aliases: &self.all_expr_aliases[file.expr_aliases],
            traits: &self.all_traits[file.traits],
            impls: &self.all_impls[file.impls],
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
) -> Vec<Module> {
    let mut jobs = VecDeque::new();
    jobs.push_back(ComputeModuleJob::new(Fs::ROOT, None, default_settings));

    let mut modules = Vec::new();

    while let Some(job) = jobs.pop_front() {
        let fs_node_id = job.fs_node_id;
        let mut module_index = job.module_index;

        let settings = module_folders
            .get(fs_node_id)
            .copied()
            .unwrap_or(job.settings);

        if module_folders.contains_key(fs_node_id) {
            module_index = Some(modules.len());
            modules.push(Module {
                settings: Some(settings),
                files: vec![],
            });
        }

        if let Some(ast_file) = files.get_mut(fs_node_id) {
            ast_file.settings = Some(settings);

            let Some(module_index) = module_index else {
                panic!(
                    "internal compiler error: This file is somehow not in a module - {}",
                    fs.get(job.fs_node_id).filename.to_string_lossy()
                );
            };

            modules[module_index].files.push(ast_file.clone());
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
                    ComputeModuleJob::new(child_fs_node_id, module_index, settings)
                }),
        );
    }

    modules
}

pub struct ComputeModuleJob {
    pub fs_node_id: FsNodeId,
    pub module_index: Option<usize>,
    pub settings: Idx<SettingsId, Settings>,
}

impl ComputeModuleJob {
    pub fn new(
        fs_node_id: FsNodeId,
        module_index: Option<usize>,
        settings: Idx<SettingsId, Settings>,
    ) -> Self {
        Self {
            fs_node_id,
            module_index,
            settings,
        }
    }
}
