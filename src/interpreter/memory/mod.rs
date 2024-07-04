mod read;
mod write;

use super::error::InterpreterError;
use crate::ir;

#[derive(Debug)]
pub struct Memory {
    heap: Vec<u8>,
    stack: Vec<u8>,
}

impl Memory {
    // NOTE: In our interpreter, both the stack and heap grow upward
    pub const STACK_OFFSET: u64 = 1024;
    pub const HEAP_OFFSET: u64 = 8 * 1024 * 1024;

    pub fn new() -> Self {
        Self {
            heap: Vec::with_capacity(1024),
            stack: Vec::with_capacity(1024),
        }
    }

    pub fn alloc_permanent(&mut self, bytes: u64) -> ir::Literal {
        ir::Literal::Unsigned64(self.alloc_permanent_raw(bytes))
    }

    pub fn alloc_permanent_raw(&mut self, bytes: u64) -> u64 {
        let address = Self::HEAP_OFFSET + u64::try_from(self.heap.len()).unwrap();
        self.heap
            .extend(std::iter::repeat(0).take(bytes.try_into().unwrap()));
        address
    }

    pub fn alloc_heap(&mut self, bytes: u64) -> ir::Literal {
        // NOTE: We don't use an actual heap allocator, since this interpreter
        // is meant for running small portions of code that probably
        // don't have a lot of temporary allocations
        self.alloc_permanent(bytes)
    }

    pub fn free_heap(&mut self, _pointer: ir::Literal) {
        // NOTE: We don't use an actual heap allocator, since this interpreter
        // is meant for running small portions of code that probably
        // don't have a lot of temporary allocations
    }

    pub fn alloc_stack(&mut self, bytes: u64) -> Result<ir::Literal, InterpreterError> {
        let raw_address = Self::STACK_OFFSET + u64::try_from(self.heap.len()).unwrap();

        if self.is_heap_address(raw_address + bytes - 1) {
            return Err(InterpreterError::StackOverflow);
        }

        let address = ir::Literal::Unsigned64(raw_address);

        self.stack
            .extend(std::iter::repeat(0).take(bytes.try_into().unwrap()));
        Ok(address)
    }

    pub fn stack_save(&mut self) -> usize {
        self.stack.len()
    }

    pub fn stack_restore(&mut self, new_len: usize) {
        self.stack.resize(new_len, 0);
    }

    pub fn is_heap_address(&self, address: u64) -> bool {
        address >= Self::HEAP_OFFSET
    }

    pub fn is_reserved_address(&self, address: u64) -> bool {
        address < Self::STACK_OFFSET
    }
}
