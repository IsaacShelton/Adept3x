use crate::server::Server;
use fingerprint::COMPILER_BUILT_AT;
use ipc_message::{IpcMessage, IpcRequest, IpcResponse};
use smol::{
    io::{self, AsyncWriteExt, BufReader, BufWriter},
    net::TcpStream,
};
use std::pin::pin;
use transport::{read_message_raw_async, write_message_raw_async};

impl Server {
    pub async fn serve(&self, mut stream: TcpStream) -> io::Result<()> {
        // We will use serialized records to communicate here
        // between us the daemon and the language server process.

        // The language server will need to be able to send us requests,
        // and we will need to handle them in a separate rt compared
        // to the normal background compilation process.

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
                IpcMessage::Response(ref _id, ref _response) => {
                    todo!("daemon: unhandled response {:?}", message)
                }
                IpcMessage::Request(ref _id, ref _request) => {
                    todo!("daemon: unhandled request {:?}", message)
                }
            }
        }
    }
}
