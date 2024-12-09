use super::{options::BuildOptions, BuildCommand};
use crate::target::{Target, TargetOs};
use std::{path::PathBuf, str::FromStr};

impl BuildCommand {
    pub fn parse(mut args: impl Iterator<Item = String>) -> Result<Self, ()> {
        let mut filename = None;
        let mut options = BuildOptions::default();

        while let Some(option) = args.next() {
            match option.as_str() {
                "-e" => options.excute_result = true,
                "--emit-ir" => options.emit_ir = true,
                "--emit-llvm-ir" => options.emit_llvm_ir = true,
                "--interpret" => {
                    options.interpret = true;
                    options.coerce_main_signature = false;
                }
                "--windows" => {
                    options.target = Target::generic_os(TargetOs::Windows);
                }
                "--mac" | "--macos" => {
                    options.target = Target::generic_os(TargetOs::Mac);
                }
                "--linux" => {
                    options.target = Target::generic_os(TargetOs::Linux);
                }
                "--freebsd" => {
                    options.target = Target::generic_os(TargetOs::FreeBsd);
                }
                "--infrastructure" => {
                    let Some(infrastructure) = args.next() else {
                        eprintln!("error: Expected infrastructure path after '--infrastructure'");
                        return Err(());
                    };

                    options.infrastructure = Some(
                        PathBuf::from_str(&infrastructure)
                            .expect("invalid non-utf-8 infrastructure path"),
                    );
                }
                _ => {
                    if filename.replace(option).is_some() {
                        eprintln!("error: Multiple paths specified");
                        return Err(());
                    }
                }
            }
        }

        let Some(filename) = filename else {
            eprintln!("error: No folder or filename specified");
            return Err(());
        };

        Ok(Self { filename, options })
    }
}
