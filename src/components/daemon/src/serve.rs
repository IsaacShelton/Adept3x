use crate::server::Server;
use fingerprint::COMPILER_BUILT_AT;
use ipc_message::{IpcMessage, IpcNotification, IpcRequest, IpcResponse};
use request::{Cache, PfIn, Rt, RtStIn, TimeoutAfterSteps};
use smol::{
    io::{self, AsyncWriteExt, BufReader, BufWriter},
    net::TcpStream,
};
use std::{num::NonZero, pin::pin};
use transport::{read_message_raw_async, write_message_raw_async};

impl Server {
    pub async fn serve(&self, mut stream: TcpStream) -> io::Result<()> {
        let idle_tracker = self.idle_tracker.clone();

        let thread = std::thread::spawn(move || {
            let mut rt = RtStIn::<PfIn>::new(Cache::load("adeptls.cache"), Some(idle_tracker));
            let query = rt.query(request::ListSymbols.into(), request::QueryMode::New);
            let result = rt.block_on(query, TimeoutAfterSteps(NonZero::new(10_000).unwrap()));

            match result {
                Ok(request::BlockOn::Complete(result)) => eprintln!("From query, got {:?}", result),
                _ => eprintln!("failed"),
            }
        });

        println!("daemon: Accepted language server connection!");

        loop {
            let reader = pin!(BufReader::new(&mut stream));

            let content = match read_message_raw_async(reader).await {
                Ok(Some(content)) => content,
                Ok(None) => continue,
                Err(error) => return Err(error),
            };

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
                IpcMessage::Request(id, IpcRequest::Initialize { fingerprint }) => {
                    if fingerprint != format!("{}", COMPILER_BUILT_AT) {
                        eprintln!("daemon: Rejecting language server built for different version");
                        let _ = stream.close();
                        return Ok(());
                    }

                    let content =
                        serde_json::to_string(&IpcMessage::Response(id, IpcResponse::Initialized))
                            .unwrap();
                    let writer = pin!(BufWriter::new(&mut stream));
                    write_message_raw_async(writer, &content).await?;
                }
                IpcMessage::Notification(IpcNotification::Exit) => {
                    eprintln!("daemon: Exit requested...");
                    break;
                }
                IpcMessage::Response(ref _id, ref _response) => {
                    todo!("daemon: unhandled response {:?}", message)
                }
                IpcMessage::Request(ref _id, ref _request) => {
                    todo!("daemon: unhandled request {:?}", message)
                }
            }
        }

        thread.join().unwrap();
        Ok(())
    }
}
