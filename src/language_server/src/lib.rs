mod dispatch;
mod handled;
mod into_lsp_response;
mod invalid;
mod logger;
mod lsp_endpoint;
mod lsp_method;
mod maybe_ready;
mod methods;
mod never_respond;
mod static_wrapper;

pub(crate) use dispatch::*;
pub(crate) use handled::*;
pub(crate) use into_lsp_response::*;
pub(crate) use invalid::*;
use lsp_connection::IdeConnection;
pub(crate) use lsp_connection::LspConnection;
pub(crate) use lsp_endpoint::*;
use lsp_message::LspMessage;
pub(crate) use lsp_method::*;
pub(crate) use maybe_ready::*;
pub(crate) use never_respond::*;
pub(crate) use static_wrapper::*;
use std::{
    io::{BufReader, ErrorKind},
    process::ExitCode,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[cfg(target_family = "windows")]
pub fn start() -> ExitCode {
    match logger::setup() {
        Ok(()) => (),
        Err(error) => {
            eprintln!("Failed to setup logger: {}", error);
            return ExitCode::FAILURE;
        }
    }

    log::error!("Language server is not supported on Windows yet");
    ExitCode::FAILURE
}

#[cfg(target_family = "unix")]
pub fn start() -> ExitCode {
    match logger::setup() {
        Ok(()) => (),
        Err(error) => {
            eprintln!("Failed to setup logger: {}", error);
            return ExitCode::FAILURE;
        }
    }

    let (ide, ide_sender) = IdeConnection::stdio();
    log::info!("Established stdio connection");

    let mut client = match daemon_init::connect() {
        Ok(daemon) => LspConnection::new(ide, ide_sender.clone(), daemon),
        Err(error) => {
            log::error!("Failed to connect to daemon - {:?}", error);
            return ExitCode::FAILURE;
        }
    };

    log::info!("Connected to daemon!");

    let daemon = client.daemon.clone();
    let should_exit = Arc::new(AtomicBool::new(false));
    let read_daemon_should_exit = should_exit.clone();

    let outgoing_thread = std::thread::spawn(move || {
        let should_exit = read_daemon_should_exit;
        daemon.stream.set_nonblocking(false).unwrap();
        daemon
            .stream
            .set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();

        while !should_exit.load(Ordering::SeqCst) {
            match LspMessage::read(&mut BufReader::new(&daemon.stream)) {
                Ok(None) => {
                    // Connection closed
                    break;
                }
                Ok(Some(message)) => {
                    // Forward the message from the daemon this language server's attached IDE
                    ide_sender.send(message);
                }
                Err(error) => {
                    if !matches!(error.kind(), ErrorKind::WouldBlock) {
                        log::error!("Error receiving message from daemon - {:?}", error);
                    }
                }
            }
        }
    });

    while let Some(message) = client.ide.wait_for_message() {
        log::info!("Received message from client: {:?}", message);

        use lsp_types::{notification::*, request::*};
        let unhandled = handle::<Initialize>(&mut client, message)
            .or_else(|message| handle::<Initialized>(&mut client, message))
            .or_else(|message| handle::<SetTrace>(&mut client, message))
            .or_else(|message| handle::<Shutdown>(&mut client, message))
            .or_else(|message| handle::<DidOpenTextDocument>(&mut client, message))
            .or_else(|message| handle::<DidChangeTextDocument>(&mut client, message))
            .or_else(|message| handle::<DocumentDiagnosticRequest>(&mut client, message))
            .err();

        if let Some(unhandled) = unhandled {
            match unhandled {
                LspMessage::Request(request) => {
                    log::warn!("Unhandled request '{}'", request.method)
                }
                LspMessage::Response(response) => {
                    client
                        .daemon
                        .send(response.into())
                        .expect("Failed to foward LSP response to daemon");
                }
                LspMessage::Notification(notification) => {
                    log::warn!("Unhandled notification '{}'", notification.method)
                }
            }
        }
    }

    should_exit.store(true, Ordering::SeqCst);
    outgoing_thread.join().unwrap();
    client.ide.join();
    log::info!("Exited");
    ExitCode::SUCCESS
}

fn handle<Method>(client: &mut LspConnection, value: LspMessage) -> Result<(), LspMessage>
where
    Static<Method>: LspEndpoint,
{
    match dispatch::<Static<Method>>(client, value) {
        DispatchResult::Handled(handled) => match handled {
            Handled::WillRespond(MaybeReady::Pending) | Handled::WontRespond => Ok(()),
            Handled::WillRespond(MaybeReady::Ready(response)) => {
                client.ide_sender.send(response.into());
                Ok(())
            }
        },
        DispatchResult::NotHandled(message) => Err(message),
    }
}
