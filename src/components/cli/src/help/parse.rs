use super::HelpCommand;

impl HelpCommand {
    pub fn parse(_: impl Iterator<Item = String>) -> Result<Self, ()> {
        Ok(Self)
    }
}
