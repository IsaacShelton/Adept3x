mod create_string;
mod print;
mod print_message;

use crate::{Executor, Progress};
pub use create_string::*;
use enum_dispatch::enum_dispatch;
pub use print::*;
pub use print_message::*;

#[enum_dispatch]
pub trait Execute {
    fn execute(self, executor: &Executor) -> Progress;
}

#[derive(Debug)]
#[enum_dispatch(Execute)]
pub enum Execution {
    CreateString(CreateString),
    Print(Print),
    PrintMessage(PrintMessage),
}
