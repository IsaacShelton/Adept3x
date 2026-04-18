use std::process::ExitCode;

pub fn compile(filename: &str) -> ExitCode {
    use lsp_message::{LspCompile, LspMessage};
    use std::{thread, time::Duration};

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let daemon = match daemon_init::connect() {
        Ok(daemon) => daemon,
        Err(error) => {
            log::error!("Failed to connect to daemon - {}", error);
            return ExitCode::FAILURE;
        }
    };

    if let Err(err) = daemon.send(LspMessage::Compile(LspCompile {
        filename: filename.into(),
    })) {
        log::error!("Failed to send compile request - {}", err);
        return ExitCode::FAILURE;
    }

    thread::sleep(Duration::from_secs(2));

    log::info!("Exited");
    ExitCode::SUCCESS
}
