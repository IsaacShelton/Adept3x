mod artifact;
mod execution;
mod executor;
mod progress;
mod task;
mod task_state;
mod truth;
mod waiting_count;
mod worker;

pub use artifact::*;
pub use execution::*;
pub use executor::*;
pub use progress::*;
pub use task::*;
pub use task_state::*;
pub use truth::*;
pub use waiting_count::*;
pub use worker::*;

fn main() {
    let executor = Executor::new(num_cpus::get().try_into().unwrap());
    let my_string = executor.push(CreateString::new("Hello World".into()));
    let my_string2 = executor.push(CreateString::new("Bye World".into()));
    executor.push(Print::new(my_string));
    executor.push(Print::new(my_string2));
    executor.push(PrintMessage::new("Goodbye.".into()));
    executor.start();
}
