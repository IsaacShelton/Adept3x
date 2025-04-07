use compiler::BuildOptions;
mod invoke;
mod parse;
mod supported_targets;

#[derive(Clone, Debug)]
pub struct BuildCommand {
    pub filename: String,
    pub options: BuildOptions,
}
