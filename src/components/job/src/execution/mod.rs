mod create_string;
mod infin;
mod print;
mod print_message;

use crate::{Executor, Progress, TaskRef};
pub use create_string::*;
use enum_dispatch::enum_dispatch;
pub use infin::Infin;
pub use print::*;
pub use print_message::*;

#[enum_dispatch]
pub trait Execute {
    fn execute(self, executor: &Executor, task_ref: TaskRef) -> Progress;
}

#[derive(Debug)]
#[enum_dispatch(Execute)]
pub enum Execution {
    CreateString(CreateString),
    Print(Print),
    PrintMessage(PrintMessage),
    Infin(Infin),
}
