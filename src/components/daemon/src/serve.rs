use crate::server::Server;
use smol::{
    io::{self, AsyncWriteExt, BufReader},
    net::TcpStream,
};
use std::pin::pin;
use transport::read_message_raw;

impl Server {
    pub async fn serve(&self, mut stream: TcpStream) -> io::Result<()> {
        // We will use serialized records to communicate here
        // between us the daemon and the language server process.

        // The language server will need to be able to send us requests,
        // and we will need to handle them in a separate rt compared
        // to the normal background compilation process.

        println!("Server received connection, and is ready to serve!");

        let reader = pin!(BufReader::new(&mut stream));
        let content = read_message_raw(reader).await;
        let _ = dbg!(content);

        // We are going to need buffered readers/writers here. Probably will need
        // to use Content-Length header for messages similar to what LSP uses.

        stream.write_all(b"hello\nworld!").await.unwrap();
        stream.close().await.unwrap();
        Ok(())
    }
}
