use super::{Value, memory::Memory};
use interpreter_api::Syscall;
use primitives::{IntegerBits, IntegerSign};

pub trait SyscallHandler {
    fn syscall<'a>(
        &mut self,
        memory: &mut Memory,
        syscall: Syscall,
        args: Vec<Value<'a>>,
    ) -> Value<'a>;
}

#[derive(Debug)]
pub struct ComptimeSystemSyscallHandler {}

impl Default for ComptimeSystemSyscallHandler {
    fn default() -> Self {
        Self {}
    }
}

fn read_cstring(memory: &Memory, value: &Value) -> String {
    let mut string = String::new();
    let mut address = value.as_u64().unwrap();

    loop {
        let c = memory
            .read_integer(address, IntegerSign::Unsigned, IntegerBits::Bits8)
            .as_u64()
            .unwrap() as u8;
        if c == 0 {
            break;
        }
        string.push(c as char);
        address += 1;
    }

    string
}

impl SyscallHandler for ComptimeSystemSyscallHandler {
    fn syscall<'a>(
        &mut self,
        _memory: &mut Memory,
        _syscall: Syscall,
        _args: Vec<Value<'a>>,
    ) -> Value<'a> {
        panic!("no syscalls are supported for comptime syscall handler")
    }
}
