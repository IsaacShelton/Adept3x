use super::HelpCommand;
use crate::cli::CliInvoke;

impl CliInvoke for HelpCommand {
    fn invoke(self) -> Result<(), ()> {
        println!("usage: adept FILENAME");
        Err(())
    }
}
