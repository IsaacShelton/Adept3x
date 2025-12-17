mod document;
mod log;
mod message;
mod methods;

use crate::message::{Message, Response};
use daemon::connect_to_daemon;
pub(crate) use document::*;
use fingerprint::COMPILER_BUILT_AT;
use ipc_message::{IpcMessage, IpcMessageId, IpcRequest, IpcResponse};
pub(crate) use log::*;
use lsp_types::{
    CompletionItem, CompletionList, CompletionResponse, DidChangeTextDocumentParams, Uri,
};
use pin_project_lite::pin_project;
use smol::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
};
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
        daemon: WriteHalf<TcpStream>,
        reader: Option<BufReader<Stdin>>,
        documents: HashMap<Uri, DocumentBody>,
    }
}

impl Server {
    pub async fn recv_from_editor(self: &mut Self) -> Option<Message> {
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

    pub async fn send_to_editor(writer_holder: &mut Option<BufWriter<Stdout>>, message: Message) {
        let mut writer = writer_holder.take().unwrap();

        // We need to unblock here, since we're using stdout
        let (result, writer) = smol::unblock(move || {
            let result = message.write_sync(&mut writer);
            (result, writer)
        })
        .await;

        *writer_holder = Some(writer);

        match result {
            Ok(()) => (),
            Err(error) => {
                eprintln!("Error: {}", error);
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

    let (read_half, write_half) = smol::io::split(daemon);

    let mut server = pin!(Server {
        did_shutdown: false,
        log,
        daemon: write_half,
        reader: Some(BufReader::new(std::io::stdin())),
        documents: Default::default(),
    });

    let mut writer = Some(BufWriter::new(std::io::stdout()));

    smol::spawn(async move {
        let mut daemon_reader = read_half;

        loop {
            let reader = pin!(smol::io::BufReader::new(&mut daemon_reader));

            let content = match read_message_raw_async(reader).await {
                Ok(Some(response)) => response,
                Ok(None) | Err(_) => return ExitCode::FAILURE,
            };

            let message = match serde_json::from_str::<IpcMessage>(&content) {
                Ok(message) => message,
                Err(_) => return ExitCode::FAILURE,
            };

            dbg!(&message);

            match message {
                IpcMessage::Request(..) => unreachable!(),
                IpcMessage::Response(_, generic_id, response) => match response {
                    IpcResponse::Initialized => {
                        Server::send_to_editor(
                            &mut writer,
                            Message::Response(Response {
                                id: generic_id.unwrap(),
                                result: Some(serde_json::to_value(methods::initialize()).unwrap()),
                                error: None,
                            }),
                        )
                        .await;
                    }
                    IpcResponse::Changed => todo!(),
                    IpcResponse::Saved => todo!(),
                    IpcResponse::Completion(items) => {
                        Server::send_to_editor(
                            &mut writer,
                            Message::Response(Response {
                                id: generic_id.unwrap(),
                                result: Some(
                                    serde_json::to_value(CompletionResponse::List(
                                        CompletionList {
                                            is_incomplete: true,
                                            items: items
                                                .iter()
                                                .map(|name| CompletionItem {
                                                    label: format!("{}", name),
                                                    ..Default::default()
                                                })
                                                .collect::<Vec<_>>(),
                                        },
                                    ))
                                    .unwrap(),
                                ),
                                error: None,
                            }),
                        )
                        .await;
                    }
                    IpcResponse::Diagnostics(items) => todo!(),
                    IpcResponse::ShuttingDown => {
                        Server::send_to_editor(
                            &mut writer,
                            Message::Response(Response {
                                id: generic_id.unwrap(),
                                result: Some(serde_json::to_value(()).unwrap()),
                                error: None,
                            }),
                        )
                        .await;
                    }
                },
                IpcMessage::Notification(..) => todo!(),
            }
        }
    })
    .detach();

    loop {
        let Some(message) = server.as_mut().recv_from_editor().await else {
            let _ = writeln!(server.log, "Breaking...");
            break;
        };

        let _ = writeln!(server.log, "Doing {:?}", &message);

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

                match request.method.as_str() {
                    "initialize" => {
                        let data = serde_json::to_string(&IpcMessage::Request(
                            Some(IpcMessageId(0)),
                            Some(id.clone()),
                            IpcRequest::Initialize {
                                fingerprint: format!("{}", COMPILER_BUILT_AT),
                            },
                        ))
                        .unwrap();

                        write_message_raw_async(server.as_mut().project().daemon, &data)
                            .await
                            .unwrap();
                        server.as_mut().project().daemon.flush().await.unwrap();
                    }
                    "textDocument/completion" => {
                        let data = serde_json::to_string(&IpcMessage::Request(
                            None,
                            Some(id.clone()),
                            IpcRequest::Completion(TextPosition(0)),
                        ))
                        .unwrap();

                        write_message_raw_async(server.as_mut().project().daemon, &data)
                            .await
                            .unwrap();
                        server.as_mut().project().daemon.flush().await.unwrap();
                    }
                    "textDocument/diagnostic" => (),
                    "shutdown" => {
                        let data = serde_json::to_string(&IpcMessage::Request(
                            None,
                            Some(id.clone()),
                            IpcRequest::Shutdown,
                        ))
                        .unwrap();

                        write_message_raw_async(server.as_mut().project().daemon, &data)
                            .await
                            .unwrap();
                        server.as_mut().project().daemon.flush().await.unwrap();
                    }
                    _ => (),
                };
            }
            Message::Response(_) => (),
            Message::Notification(notif) => match notif.method.as_str() {
                "initialized" => (),
                "textDocument/didChange" => {
                    let params =
                        serde_json::from_value::<DidChangeTextDocumentParams>(notif.params)
                            .unwrap();

                    dbg!(params);

                    /*
                    notification.params

                        write_message_raw_async(server.as_mut().project().daemon, &data)
                            .await
                            .unwrap();
                        server.as_mut().project().daemon.flush().await.unwrap();
                    */
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

    ExitCode::SUCCESS
}
