use diagnostics::{Diagnostics, WarningDiagnostic};
use target::{Target, TargetArch, TargetOs};

pub fn warn_if_unsupported_target(target: &Target, diagnostics: &Diagnostics) {
    if target.arch().is_none() {
        diagnostics.push(WarningDiagnostic::plain(
            "Target architecture is not supported, falling back to best guess",
        ));
    }

    if target.os().is_none() {
        diagnostics.push(WarningDiagnostic::plain(
            "Target os is not supported, falling back to best guess",
        ));
    }

    match target.os().zip(target.arch()) {
        Some((TargetOs::Windows, TargetArch::X86_64)) => (),
        Some((TargetOs::Windows, TargetArch::Aarch64)) => (),
        Some((TargetOs::Mac, TargetArch::X86_64)) => (),
        Some((TargetOs::Mac, TargetArch::Aarch64)) => (),
        Some((TargetOs::Linux, TargetArch::X86_64)) => (),
        Some((TargetOs::Linux, TargetArch::Aarch64)) => (),
        Some((TargetOs::FreeBsd, TargetArch::X86_64)) => (),
        None => (),
        #[allow(unreachable_patterns)]
        _ => {
            diagnostics.push(WarningDiagnostic::plain(
                "Host os/architecture configuration is not officially supported, taking best guess",
            ));
        }
    }
}
