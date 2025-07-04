use super::{Value, memory::Memory};
use crate::value::ValueKind;
use compiler_version::AdeptVersion;
use interpreter_api::Syscall;
use num::FromPrimitive;
use num_derive::FromPrimitive;
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

pub trait SyscallHandler {
    fn syscall<'a>(
        &mut self,
        memory: &mut Memory,
        syscall: Syscall,
        args: Vec<Value<'a>>,
    ) -> Value<'a>;
}

#[derive(Debug)]
pub struct Project {
    pub name: String,
    pub kind: ProjectKind,
}

#[derive(Debug, FromPrimitive)]
pub enum ProjectKind {
    ConsoleApp = 0,
    WindowedApp = 1,
}

#[derive(Debug)]
pub struct BuildSystemSyscallHandler {
    pub projects: Vec<Project>,
    pub version: Option<AdeptVersion>,
    pub link_filenames: HashSet<String>,
    pub link_frameworks: HashSet<String>,
    pub debug_skip_merging_helper_exprs: bool,
    pub imported_namespaces: Vec<Box<str>>,
    pub assume_int_at_least_32_bits: bool,
    pub namespace_to_dependency: HashMap<String, Vec<String>>,
}

impl Default for BuildSystemSyscallHandler {
    fn default() -> Self {
        Self {
            projects: vec![],
            version: None,
            link_filenames: HashSet::new(),
            link_frameworks: HashSet::new(),
            debug_skip_merging_helper_exprs: false,
            imported_namespaces: vec![],
            assume_int_at_least_32_bits: true,
            namespace_to_dependency: HashMap::new(),
        }
    }
}

fn read_cstring(memory: &Memory, value: &Value) -> String {
    let mut string = String::new();
    let mut address = value.as_u64().unwrap();

    loop {
        let c = memory.read_u8(address).as_u64().unwrap() as u8;
        if c == 0 {
            break;
        }
        string.push(c as char);
        address += 1;
    }

    string
}

impl SyscallHandler for BuildSystemSyscallHandler {
    fn syscall<'a>(
        &mut self,
        memory: &mut Memory,
        syscall: Syscall,
        args: Vec<Value<'a>>,
    ) -> Value<'a> {
        match syscall {
            Syscall::Println => {
                assert_eq!(args.len(), 1);
                println!("{}", read_cstring(memory, &args[0]));
                ValueKind::Literal(ir::Literal::Void).untainted()
            }
            Syscall::BuildAddProject => {
                assert_eq!(args.len(), 2);
                let name = read_cstring(memory, &args[0]);
                let kind = ProjectKind::from_u64(args[1].as_u64().unwrap()).unwrap();
                self.projects.push(Project { name, kind });
                ValueKind::Literal(ir::Literal::Void).untainted()
            }
            Syscall::BuildLinkFilename => {
                assert_eq!(args.len(), 1);
                self.link_filenames.insert(read_cstring(memory, &args[0]));
                ValueKind::Literal(ir::Literal::Void).untainted()
            }
            Syscall::BuildLinkFrameworkName => {
                assert_eq!(args.len(), 1);
                self.link_frameworks.insert(read_cstring(memory, &args[0]));
                ValueKind::Literal(ir::Literal::Void).untainted()
            }
            Syscall::BuildSetAdeptVersion => {
                assert_eq!(args.len(), 1);

                let version_string = read_cstring(memory, &args[0]);
                if let Ok(version) = AdeptVersion::from_str(&version_string) {
                    self.version = Some(version);
                } else {
                    println!(
                        "warning: Ignoring unrecognized Adept version '{}'",
                        version_string
                    );
                }

                ValueKind::Literal(ir::Literal::Void).untainted()
            }
            Syscall::Experimental => {
                assert_eq!(args.len(), 1);

                let option = read_cstring(memory, &args[0]);

                match option.as_str() {
                    "debug_skip_merging_helper_exprs" => {
                        self.debug_skip_merging_helper_exprs = true;
                    }
                    _ => {
                        println!(
                            "warning: Ignoring unrecognized experimental setting '{}'",
                            option
                        );
                    }
                }

                ValueKind::Literal(ir::Literal::Void).untainted()
            }
            Syscall::ImportNamespace => {
                assert_eq!(args.len(), 1);
                self.imported_namespaces
                    .push(read_cstring(memory, &args[0]).into_boxed_str());
                ValueKind::Literal(ir::Literal::Void).untainted()
            }
            Syscall::DontAssumeIntAtLeast32Bits => {
                assert_eq!(args.len(), 0);
                self.assume_int_at_least_32_bits = false;
                ValueKind::Literal(ir::Literal::Void).untainted()
            }
            Syscall::UseDependency => {
                assert_eq!(args.len(), 2);

                let as_namespace = read_cstring(memory, &args[0]);
                let dependency_name = read_cstring(memory, &args[1]);

                self.namespace_to_dependency
                    .entry(as_namespace)
                    .or_default()
                    .push(dependency_name);

                #[allow(unreachable_code)]
                ValueKind::Literal(ir::Literal::Void).untainted()
            }
        }
    }
}
