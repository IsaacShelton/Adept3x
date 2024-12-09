mod build;
mod help;
mod new;

use build::BuildCommand;
pub use build::BuildOptions;
use enum_dispatch::enum_dispatch;
use help::HelpCommand;
use new::NewCommand;

#[enum_dispatch(CliInvoke)]
#[derive(Clone, Debug)]
pub enum CliCommand {
    Help(HelpCommand),
    Build(BuildCommand),
    New(NewCommand),
}

impl CliCommand {
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
pub trait CliInvoke {
    fn invoke(self) -> Result<(), ()>;
}
