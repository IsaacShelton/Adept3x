mod file;
mod function;
mod linkset;
mod namespace;
mod namespace_items;
mod pragma;
mod structure;
mod when;

pub use file::{ProcessFile, RequireFileHeader};
pub use function::ProcessFunction;
pub use linkset::ProcessLinkset;
pub use namespace::ProcessNamespace;
pub use namespace_items::ProcessNamespaceItems;
pub use pragma::ProcessPragma;
pub use structure::ProcessStructure;
pub use when::ProcessWhen;

/*
    Within this codebase, "processing" means:

    For runtime code:
    - Fully resolving everything that is needed for it,
      up to and including generating the LLVM IR or other
      backend IR that will be used to generate an object file.

    For comptime code:
    - Generating all heads used to search and connect items together.
    - To run comptime code, only the required items are compiled lazily
      when requested/invoked.
    - Note that this can be self-referential, in order to generate the
      heads of some symbols, other comptime code may have to be run first,
      which itself requires some heads to be processed (to varying levels),
      hence why.
*/
