mod build;
mod help;
mod new;

use build::BuildCommand;
use enum_dispatch::enum_dispatch;
use help::HelpCommand;
use new::NewCommand;

#[enum_dispatch(Invoke)]
#[derive(Clone, Debug)]
pub enum Command {
    Help(HelpCommand),
    Build(BuildCommand),
    New(NewCommand),
}

impl Command {
    pub fn parse() -> Result<Self, ()> {
        let mut args = std::env::args().skip(1).peekable();

        match args.peek().map(String::as_str) {
            Some("-h" | "--help") | None => HelpCommand::parse(args).map(Self::from),
            Some("new") => NewCommand::parse(args).map(Self::from),
            _ => BuildCommand::parse(args).map(Self::from),
        }
    }
}

#[enum_dispatch]
pub trait Invoke {
    fn invoke(self) -> Result<(), ()>;
}
