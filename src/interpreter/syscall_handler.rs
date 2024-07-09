use super::{memory::Memory, Value};
use crate::ir::{self, InterpreterSyscallKind};
use num::FromPrimitive;
use num_derive::FromPrimitive;

pub trait SyscallHandler {
    fn syscall<'a>(
        &mut self,
        memory: &mut Memory,
        syscall: InterpreterSyscallKind,
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

#[derive(Debug, Default)]
pub struct BuildSystemSyscallHandler {
    pub projects: Vec<Project>,
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
        syscall: InterpreterSyscallKind,
        args: Vec<Value<'a>>,
    ) -> Value<'a> {
        match syscall {
            ir::InterpreterSyscallKind::Println => {
                assert_eq!(args.len(), 1);
                println!("{}", read_cstring(memory, &args[0]));
                Value::Literal(ir::Literal::Void)
            }
            ir::InterpreterSyscallKind::BuildAddProject => {
                assert_eq!(args.len(), 2);
                let name = read_cstring(memory, &args[0]);
                let kind = ProjectKind::from_u64(args[1].as_u64().unwrap()).unwrap();
                self.projects.push(Project { name, kind });
                Value::Literal(ir::Literal::Void)
            }
        }
    }
}
