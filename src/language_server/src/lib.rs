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
pub(crate) use lsp_connection::LspConnection;
pub(crate) use lsp_endpoint::*;
use lsp_message::LspMessage;
pub(crate) use lsp_method::*;
pub(crate) use maybe_ready::*;
pub(crate) use never_respond::*;
pub(crate) use static_wrapper::*;
use std::process::ExitCode;

pub fn start() -> ExitCode {
    match logger::setup() {
        Ok(()) => (),
        Err(error) => {
            eprintln!("Failed to setup logger: {}", error);
            return ExitCode::FAILURE;
        }
    }

    let mut client = LspConnection::stdio();
    log::info!("Established stdio connection");

    match daemon_init::connect() {
        Ok(connection) => client.daemon = Some(connection),
        Err(error) => {
            log::error!("Failed to connect to daemon - {:?}", error);
            return ExitCode::FAILURE;
        }
    }

    log::info!("Connected to daemon!");

    while let Some(message) = client.wait_for_message() {
        log::info!("Received message from client: {:?}", message);

        use lsp_types::{notification::*, request::*};
        let unhandled = handle::<Initialize>(&mut client, message)
            .or_else(|message| handle::<Initialized>(&mut client, message))
            .or_else(|message| handle::<SetTrace>(&mut client, message))
            .or_else(|message| handle::<Shutdown>(&mut client, message))
            .or_else(|message| handle::<DidOpenTextDocument>(&mut client, message))
            .or_else(|message| handle::<DidChangeTextDocument>(&mut client, message))
            .err();

        if let Some(unhandled) = unhandled {
            match unhandled {
                LspMessage::Request(request) => {
                    log::warn!("Unhandled request '{}'", request.method)
                }
                LspMessage::Response(response) => {
                    log::warn!("Unhandled response from client for id {}", response.id)
                }
                LspMessage::Notification(notification) => {
                    log::warn!("Unhandled notification '{}'", notification.method)
                }
            }
        }
    }

    log::info!("Joining threads");
    client.join();
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
                client.send(response.into());
                Ok(())
            }
        },
        DispatchResult::NotHandled(message) => Err(message),
    }
}
