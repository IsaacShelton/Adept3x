use super::HelpCommand;
use crate::Invoke;

impl Invoke for HelpCommand {
    fn invoke(self) -> Result<(), ()> {
        println!("usage: adept FILENAME");
        Err(())
    }
}
