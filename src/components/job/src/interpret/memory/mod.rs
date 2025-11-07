mod read;
mod write;

use super::error::InterpreterError;
use crate::ir;
use bit_vec::BitVec;
use data_units::ByteUnits;

#[derive(Debug)]
pub struct Memory {
    heap: Vec<u8>,
    stack: Vec<u8>,
    // Track whether values are derived from the compilation host's sizeof.
    // We do this to raise errors and make it difficult to
    // unintentionally transfer values derived from the compilation host's sizeof
    // to runtime code, as runtime code should never depend on implementation details
    // of the virtual machine used to run compile-time code.
    // The sizeof a value for the compilation host is not guaranteed to be
    // the same as the target platform, which is why we prevent accidental leakage
    // (even convoluted cases) through this tracking.
    heap_tainted_by_comptime_sizeof: BitVec,
    stack_tainted_by_comptime_sizeof: BitVec,
}

impl Memory {
    // NOTE: In our interpreter, both the stack and heap grow upward
    pub const STACK_OFFSET: u64 = 1024;
    pub const HEAP_OFFSET: u64 = 8 * 1024 * 1024;

    pub fn new() -> Self {
        Self {
            heap: Vec::with_capacity(1024),
            stack: Vec::with_capacity(1024),
            heap_tainted_by_comptime_sizeof: BitVec::with_capacity(1024),
            stack_tainted_by_comptime_sizeof: BitVec::with_capacity(1024),
        }
    }

    pub fn alloc_permanent<'env>(&mut self, bytes: ByteUnits) -> ir::Literal<'env> {
        ir::Literal::new_u64(self.alloc_permanent_raw(bytes))
    }

    pub fn alloc_permanent_raw(&mut self, bytes: ByteUnits) -> u64 {
        let address = Self::HEAP_OFFSET + u64::try_from(self.heap.len()).unwrap();
        let bytes = usize::try_from(bytes.bytes()).unwrap();
        self.heap.extend(std::iter::repeat(0).take(bytes));
        self.heap_tainted_by_comptime_sizeof.grow(bytes, false);
        address
    }

    pub fn alloc_heap<'env>(&mut self, bytes: ByteUnits) -> ir::Literal<'env> {
        // NOTE: We don't use an actual heap allocator, since this interpreter
        // is meant for running small portions of code that probably
        // don't have a lot of temporary allocations
        self.alloc_permanent(bytes)
    }

    pub fn free_heap<'env>(&mut self, _pointer: ir::Literal<'env>) {
        // NOTE: We don't use an actual heap allocator, since this interpreter
        // is meant for running small portions of code that probably
        // don't have a lot of temporary allocations
    }

    pub fn alloc_stack<'env>(
        &mut self,
        bytes: ByteUnits,
    ) -> Result<ir::Literal<'env>, InterpreterError> {
        let raw_address = Self::STACK_OFFSET + u64::try_from(self.stack.len()).unwrap();

        if self.is_heap_address(raw_address + bytes.bytes() - 1) {
            return Err(InterpreterError::StackOverflow);
        }

        let address = ir::Literal::new_u64(raw_address);
        let bytes = usize::try_from(bytes.bytes()).unwrap();
        self.stack.extend(std::iter::repeat(0).take(bytes));
        self.stack_tainted_by_comptime_sizeof.grow(bytes, false);
        Ok(address)
    }

    pub fn stack_save(&self) -> usize {
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
