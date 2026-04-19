use lsp_message::{ExtCompile, LspMessage};
use request::{Aft, BlockOn, UnwrapAft};
use std::process::ExitCode;

pub fn compile(filename: &str) -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let daemon = match daemon_init::connect() {
        Ok(daemon) => daemon,
        Err(error) => {
            log::error!("Failed to connect to daemon - {}", error);
            return ExitCode::FAILURE;
        }
    };

    if let Err(err) = LspMessage::send(
        &daemon,
        LspMessage::ExtCompile(ExtCompile {
            ext_compile: filename.into(),
        }),
    ) {
        log::error!("Failed to send compile request - {}", err);
        return ExitCode::FAILURE;
    }

    let message = LspMessage::recv(&daemon);

    match message {
        Ok(Some(LspMessage::ExtAft(aft_result))) => match aft_result.ext_aft {
            BlockOn::Complete(Some(complete)) => {
                let aft = Aft::from(complete);
                let ret = request::ListSymbols::unwrap_aft(aft);

                for name in ret.value.iter() {
                    println!(" - {name}");
                }
            }
            BlockOn::Complete(None) => {
                unreachable!("result not serializable");
            }
            BlockOn::Cyclic => {
                log::info!("Cyclic");
            }
            BlockOn::Diverges => {
                log::info!("Diverges");
            }
            BlockOn::TimedOut => {
                log::info!("Timed out");
            }
        },
        Ok(Some(LspMessage::ExtError(ext_error))) => {
            eprintln!("ERROR: {}", ext_error.ext_error);
        }
        Ok(_) => {
            log::error!("Driver received invalid response");
        }
        Err(error) => {
            log::error!("Failed to receive response {}", error)
        }
    }

    log::info!("Exited");
    ExitCode::SUCCESS
}
