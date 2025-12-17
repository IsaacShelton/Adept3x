use crate::server::Server;
use fingerprint::COMPILER_BUILT_AT;
use fluent_uri::Uri;
use ipc_message::{
    GenericRequestId, IpcMessage, IpcMessageId, IpcNotification, IpcRequest, IpcResponse,
};
use request::{Cache, PfIn, Req, Rt, RtStIn, TimeoutAfterSteps, UnwrapAft, WithErrors};
use smol::{
    io::{self, AsyncWriteExt, BufReader, BufWriter},
    lock::Mutex,
    net::TcpStream,
    stream::StreamExt,
};
use std::{collections::HashMap, num::NonZero, pin::pin, sync::Arc};
use transport::{read_message_raw_async, write_message_raw_async};

#[derive(Default)]
pub struct LanguageClient {
    pub documents: Mutex<HashMap<Uri<String>, Document>>,
}

#[derive(Clone, Debug)]
pub struct Document {
    pub content: String,
}

impl Server {
    pub async fn serve(&self, mut stream: TcpStream) -> io::Result<()> {
        let idle_tracker = self.idle_tracker.clone();
        let stream_writer = stream.clone();
        let client = Arc::new(LanguageClient::default());

        // Channel for sending requests off to be processed
        let (tx_req, rx_req) =
            smol::channel::bounded::<(Option<IpcMessageId>, Option<GenericRequestId>, IpcRequest)>(
                16,
            );

        // Thread for core processing
        let thread = std::thread::spawn(move || {
            let mut rx_req = pin!(rx_req);
            let mut rt = RtStIn::<PfIn>::new(Cache::load("adeptls.cache"), Some(idle_tracker));

            let mut stream_writer = stream_writer;

            loop {
                let Some((id, generic_id, ipc_request)) =
                    smol::block_on(async { rx_req.next().await })
                else {
                    break;
                };

                let req: Req = match &ipc_request {
                    IpcRequest::Initialize { .. } => {
                        unreachable!("Core processing by daemon does not handle initialization")
                    }
                    IpcRequest::Shutdown => {
                        let response =
                            IpcMessage::Response(id, generic_id, IpcResponse::ShuttingDown);
                        let data = serde_json::to_string(&response).unwrap();
                        smol::block_on(async {
                            let writer = BufWriter::new(&mut stream_writer);
                            let writer = pin!(writer);
                            let _ = write_message_raw_async(writer, &data).await;
                        });
                        break;
                    }
                    IpcRequest::Completion(text_position) => request::ListSymbols.into(),
                    IpcRequest::Diagnostics(ipc_file) => todo!(),
                };

                let query = rt.query(req, request::QueryMode::New);
                let result = rt.block_on(query, TimeoutAfterSteps(NonZero::new(10_000).unwrap()));

                let aft = match result {
                    Ok(request::BlockOn::Complete(result)) => {
                        eprintln!("From query {:?}, got {:?}", id, result);
                        result
                    }
                    _ => {
                        eprintln!("Failed to complete query in time limit");
                        break;
                    }
                };

                match ipc_request {
                    IpcRequest::Initialize { .. } => unreachable!(),
                    IpcRequest::Shutdown => unreachable!(),
                    IpcRequest::Completion(_text_position) => {
                        let WithErrors { value: names, .. } = request::ListSymbols::unwrap_aft(aft);

                        let response =
                            IpcMessage::Response(id, generic_id, IpcResponse::Completion(names));
                        let data = serde_json::to_string(&response).unwrap();
                        if smol::block_on(async {
                            let writer = BufWriter::new(&mut stream_writer);
                            let writer = pin!(writer);
                            write_message_raw_async(writer, &data).await
                        })
                        .is_err()
                        {
                            break;
                        }
                    }
                    IpcRequest::Diagnostics(ipc_file) => todo!(),
                }
            }
        });

        eprintln!("daemon: Accepted language server connection!");

        // Receive messages from language server
        loop {
            let reader = pin!(BufReader::new(&mut stream));

            eprintln!("daemon: Waiting for message...");
            let content = match read_message_raw_async(reader).await {
                Ok(Some(content)) => content,
                Ok(None) => break,
                Err(error) => return Err(error),
            };

            eprintln!("daemon: Processing message...");
            let message = match serde_json::from_str::<IpcMessage>(&content) {
                Ok(message) => message,
                Err(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Failed to deserialize in expected format",
                    ));
                }
            };

            match message {
                IpcMessage::Request(id, generic_id, IpcRequest::Initialize { fingerprint }) => {
                    if fingerprint != format!("{}", COMPILER_BUILT_AT) {
                        eprintln!("daemon: Rejecting language server built for different version");
                        let _ = stream.close();
                        return Ok(());
                    }

                    let content = serde_json::to_string(&IpcMessage::Response(
                        id,
                        generic_id,
                        IpcResponse::Initialized,
                    ))
                    .unwrap();
                    let writer = pin!(BufWriter::new(&mut stream));
                    write_message_raw_async(writer, &content).await?;
                }
                IpcMessage::Notification(_, IpcNotification::DidChange(ipc_file, edits)) => {
                    todo!("daemon: notif {:?} {:?}", ipc_file, edits)
                }
                IpcMessage::Notification(_, IpcNotification::DidOpen(..)) => {
                    todo!("daemon: did open")
                }
                IpcMessage::Notification(_, IpcNotification::DidSave(..)) => {
                    todo!("daemon: did save")
                }
                IpcMessage::Notification(_, IpcNotification::Exit) => {
                    eprintln!("daemon: Exit requested...");
                    break;
                }
                IpcMessage::Response(ref _id, ref _generic_id, ref _response) => {
                    todo!("daemon: unhandled response {:?}", message)
                }
                IpcMessage::Request(id, generic_id, request) => {
                    tx_req.send((id, generic_id, request)).await.unwrap();
                }
            }
        }

        eprintln!("daemon: Closing language server connection...");
        tx_req.close();
        thread.join().unwrap();
        eprintln!("daemon: Closed language server connection...");
        Ok(())
    }
}
