mod invoke;
mod options;
mod parse;
mod supported_targets;

pub use options::BuildOptions;

#[derive(Clone, Debug)]
pub struct BuildCommand {
    pub filename: String,
    pub options: BuildOptions,
}
