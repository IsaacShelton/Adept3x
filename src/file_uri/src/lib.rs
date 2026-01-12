/*
    ==========================  file_uri/src/lib.rs  ==========================
    Converts a URI to an actual file path (or back), taking special precautions
    to avoid several pitfalls present on Windows.

    Forked from MIT-licensed odoo implementation.
    ---------------------------------------------------------------------------
*/

mod decode;
mod encode;

pub use decode::DecodeFileUri;
pub use encode::EncodeFileUri;
