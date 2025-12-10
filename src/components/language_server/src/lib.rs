mod document;
mod log;
mod message;
mod methods;

use crate::message::{Message, Response};
use daemon::connect_to_daemon;
pub(crate) use document::*;
use fingerprint::COMPILER_BUILT_AT;
use ipc_message::{Ipc, IpcMessageId, IpcRequest};
pub(crate) use log::*;
use lsp_types::Uri;
use std::{
    collections::HashMap,
    io::{BufReader, BufWriter, Stdin, Stdout, Write},
    net::TcpStream,
    process::ExitCode,
};

pub struct Server {
    did_shutdown: bool,
    log: Logger,
    daemon: TcpStream,
    reader: BufReader<Stdin>,
    writer: BufWriter<Stdout>,
    documents: HashMap<Uri, DocumentBody>,
}

impl Server {
    pub fn recv_message(&mut self) -> Option<Message> {
        match Message::read(&mut self.reader) {
            Ok(message) => message,
            Err(error) => {
                let _ = writeln!(self.log, "Error: {}", error);
                None
            }
        }
    }

    pub fn send_message(&mut self, message: Message) {
        let _ = message.write(&mut self.writer);
    }
}

pub fn start() -> ExitCode {
    let mut log =
        Logger::new_with_file("adept_language_server.log").expect("failed to create log file");
    let _ = writeln!(log, "Log file created");

    let Ok(daemon) = connect_to_daemon() else {
        return ExitCode::FAILURE;
    };

    let mut server = Server {
        did_shutdown: false,
        log,
        daemon,
        reader: BufReader::new(std::io::stdin()),
        writer: BufWriter::new(std::io::stdout()),
        documents: Default::default(),
    };

    serde_json::to_writer(
        &mut server.daemon,
        &Ipc::Request(
            IpcMessageId(0),
            IpcRequest::Initialize {
                fingerprint: format!("{}", COMPILER_BUILT_AT),
            },
        ),
    )
    .unwrap();
    writeln!(&mut server.daemon, "").unwrap();
    server.daemon.flush().unwrap();

    loop {
        let Some(message) = server.recv_message() else {
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
                    server.send_message(response);
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
