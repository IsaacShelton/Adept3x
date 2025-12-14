mod document;
mod log;
mod message;
mod methods;

use crate::message::{Message, Response};
use daemon::connect_to_daemon;
pub(crate) use document::*;
use fingerprint::COMPILER_BUILT_AT;
use ipc_message::{IpcMessage, IpcMessageId, IpcRequest};
pub(crate) use log::*;
use lsp_types::Uri;
use pin_project_lite::pin_project;
use smol::{io::AsyncWriteExt, net::TcpStream};
use std::{
    collections::HashMap,
    io::{BufReader, BufWriter, Stdin, Stdout, Write},
    pin::pin,
    process::ExitCode,
};
use text_edit::TextPosition;
use transport::{read_message_raw_async, write_message_raw_async};

pin_project! {
    pub struct Server {
        did_shutdown: bool,
        log: Logger,
        #[pin]
        daemon: TcpStream,
        reader: Option<BufReader<Stdin>>,
        writer: Option<BufWriter<Stdout>>,
        documents: HashMap<Uri, DocumentBody>,
    }
}

impl Server {
    pub async fn recv_message(self: &mut Self) -> Option<Message> {
        let mut reader = self.reader.take()?;

        // We need to unblock here, since we're using stdin
        let (reader, result) = smol::unblock(move || {
            let result = Message::read_sync(&mut reader);
            (reader, result)
        })
        .await;

        self.reader = Some(reader);

        match result {
            Ok(message) => message,
            Err(error) => {
                let _ = writeln!(self.log, "Error: {}", error);
                None
            }
        }
    }

    pub async fn send_message(&mut self, message: Message) {
        let mut writer = self.writer.take().unwrap();

        // We need to unblock here, since we're using stdout
        let (writer, result) = smol::unblock(move || {
            let result = message.write_sync(&mut writer);
            (writer, result)
        })
        .await;

        self.writer = Some(writer);

        match result {
            Ok(()) => (),
            Err(error) => {
                let _ = writeln!(self.log, "Error: {}", error);
            }
        }
    }
}

pub async fn start() -> ExitCode {
    let mut log =
        Logger::new_with_file("adept_language_server.log").expect("failed to create log file");
    let _ = writeln!(log, "Log file created");

    let Ok(daemon) = connect_to_daemon().await else {
        let _ = writeln!(
            log,
            "Could not establish connection to project daemon process"
        );
        return ExitCode::FAILURE;
    };

    let mut server = pin!(Server {
        did_shutdown: false,
        log,
        daemon,
        reader: Some(BufReader::new(std::io::stdin())),
        writer: Some(BufWriter::new(std::io::stdout())),
        documents: Default::default(),
    });

    let data = serde_json::to_string(&IpcMessage::Request(
        IpcMessageId(0),
        IpcRequest::Initialize {
            fingerprint: format!("{}", COMPILER_BUILT_AT),
        },
    ))
    .unwrap();
    write_message_raw_async(server.as_mut().project().daemon, &data)
        .await
        .unwrap();
    server.as_mut().project().daemon.flush().await.unwrap();
    let reader = pin!(smol::io::BufReader::new(server.as_mut().project().daemon));
    let content = match read_message_raw_async(reader).await {
        Ok(Some(response)) => response,
        Ok(None) | Err(_) => return ExitCode::FAILURE,
    };

    let message = match serde_json::from_str::<IpcMessage>(&content) {
        Ok(message) => message,
        Err(_) => return ExitCode::FAILURE,
    };

    let _ = dbg!(message);

    let data = serde_json::to_string(&IpcMessage::Request(
        IpcMessageId(0),
        IpcRequest::Completion(TextPosition(0)),
    ))
    .unwrap();
    write_message_raw_async(server.as_mut().project().daemon, &data)
        .await
        .unwrap();
    server.as_mut().project().daemon.flush().await.unwrap();

    loop {
        let Some(message) = server.as_mut().recv_message().await else {
            continue;
        };

        match &message {
            Message::Request(request) => {
                let _ = writeln!(
                    server.log,
                    "Received request {}: {:?}",
                    request.method.as_str(),
                    message
                );
            }
            Message::Response(_) => {
                let _ = writeln!(server.log, "Received response {:?}", message);
            }
            Message::Notification(notification) => {
                let _ = writeln!(
                    server.log,
                    "Received notification {}: {:?}",
                    notification.method.as_str(),
                    message
                );

                if notification.method.as_str() == "exit" {
                    let _ = writeln!(server.log, "Exit Requested, exiting...");
                }
            }
        }

        let _ = server.log.flush();

        match message {
            Message::Request(request) => {
                let id = request.id.clone();

                let result = match request.method.as_str() {
                    "initialize" => {
                        Some(serde_json::to_value(methods::initialize(request)).unwrap())
                    }
                    "textDocument/completion" => Some(
                        serde_json::to_value(methods::text_document::completion(&server, request))
                            .unwrap(),
                    ),
                    "textDocument/diagnostic" => Some(
                        serde_json::to_value(methods::text_document::diagnostic(&server, request))
                            .unwrap(),
                    ),
                    "shutdown" => {
                        Some(serde_json::to_value(methods::shutdown(&mut server, request)).unwrap())
                    }
                    _ => None,
                };

                let response = result.map(|result| {
                    Message::Response(Response {
                        id,
                        result: Some(result),
                        error: None,
                    })
                });

                if let Some(response) = response {
                    let _ = writeln!(server.log, "Sending message: {:?}", response);
                    server.as_mut().send_message(response).await;
                }
            }
            Message::Response(_) => (),
            Message::Notification(notification) => match notification.method.as_str() {
                "initialized" => (),
                "textDocument/didChange" => {
                    methods::text_document::did_change(&mut server, notification);
                }
                "exit" => {
                    return if server.did_shutdown {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    };
                }
                _ => (),
            },
        }
    }
}
