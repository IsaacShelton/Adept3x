use lsp_message::{LspCompile, LspMessage};
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
        LspMessage::Compile(LspCompile {
            filename: filename.into(),
        }),
    ) {
        log::error!("Failed to send compile request - {}", err);
        return ExitCode::FAILURE;
    }

    let message = LspMessage::recv(&daemon);

    log::info!("Got response {:?}", message);

    log::info!("Exited");
    ExitCode::SUCCESS
}
