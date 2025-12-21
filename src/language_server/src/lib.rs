mod into_lsp_response;
mod invalid;
mod logger;
mod lsp_method;
mod lsp_method_endpoint;
mod lsp_method_handled;
mod maybe_ready;
mod methods;
mod never_respond;
mod static_wrapper;

pub use into_lsp_response::*;
pub use invalid::*;
use lsp_connection::{LspConnection, LspMessage};
pub use lsp_method::*;
pub use lsp_method_endpoint::*;
pub use lsp_method_handled::*;
pub use maybe_ready::*;
pub use never_respond::*;
pub use static_wrapper::*;
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

    while let Some(message) = client.wait_for_message() {
        log::info!("Received message from client: {:?}", message);

        use lsp_types::{notification::*, request::*};
        let unhandled = handle::<Initialize>(&mut client, message)
            .or_else(|message| handle::<Initialized>(&mut client, message))
            .or_else(|message| handle::<Shutdown>(&mut client, message))
            .err();

        if let Some(unhandled) = unhandled {
            match unhandled {
                LspMessage::Request(request) => {
                    log::info!("Unhandled request '{}'", request.method)
                }
                LspMessage::Response(response) => {
                    log::info!("Unhandled response from client for id {}", response.id)
                }
                LspMessage::Notification(notification) => {
                    log::info!("Unhandled notification '{}'", notification.method)
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
    Static<Method>: LspRequestEndpoint,
{
    match dispatch::<Static<Method>>(client, value) {
        LspMethodDispatchResult::Handled(handled) => match handled {
            LspMethodHandled::WillRespond(MaybeReady::Pending) | LspMethodHandled::WontRespond => {
                Ok(())
            }
            LspMethodHandled::WillRespond(MaybeReady::Ready(response)) => {
                client.send(response.into());
                Ok(())
            }
        },
        LspMethodDispatchResult::NotHandled(message) => Err(message),
    }
}
