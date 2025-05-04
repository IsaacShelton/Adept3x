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

/*
fn main() -> Result<(), ()> {
    let filename = "/Users/isaac/Projects/Adept3x/adept/tests/success/modules_simple".to_string();
    let options = BuildOptions {
        execute_result: true,
        ..Default::default()
    };

    cli::Command::Build(BuildCommand { filename, options }).invoke()
}

fn main() {
    let executor = MainExecutor::new(num_cpus::get().try_into().unwrap());
    let my_string = executor.push(CreateString::new("Hello World".into()));
    let my_string2 = executor.push(CreateString::new("Bye World".into()));
    executor.push(Print::new(my_string));
    executor.push(Print::new(my_string2));
    executor.push(PrintMessage::new("Goodbye.".into()));

    // Testing cyclic dependencies
    executor.push(Infin::new());

    let stats = executor.start();

    let show_stats = false;

    if stats.num_scheduled != stats.num_completed {
        let num_cyclic = stats.num_scheduled - stats.num_completed;

        if num_cyclic == 1 {
            println!("error: {} cyclic dependency found!", num_cyclic);
        } else {
            println!("error: {} cyclic dependencies found!", num_cyclic);
        }
    } else if show_stats {
        println!("Tasks: {}/{}", stats.num_completed, stats.num_scheduled,);
        println!("Queued: {}/{}", stats.num_cleared, stats.num_queued,);
    }
}
*/
