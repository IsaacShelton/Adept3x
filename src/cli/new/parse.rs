use super::NewCommand;

impl NewCommand {
    pub fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, ()> {
        args.next().expect("skip over 'new' command keyword");

        let (Some(project_name), None) = (args.next(), args.next()) else {
            println!("adept new <PROJECT_NAME>");
            return Err(());
        };

        Ok(Self { project_name })
    }
}
