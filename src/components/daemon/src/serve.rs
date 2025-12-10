use crate::server::Server;
use smol::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

impl Server {
    pub async fn serve(&self, mut stream: TcpStream) -> io::Result<()> {
        // We will use serialized records to communicate here
        // between us the daemon and the language server process.

        // The language server will need to be able to send us requests,
        // and we will need to handle them in a separate rt compared
        // to the normal background compilation process.

        println!("Server received connection, and is ready to serve!");

        let mut reader = BufReader::new(&mut stream);
        let mut buf = String::new();
        reader.read_line(&mut buf).await.unwrap();
        dbg!(buf);

        // We are going to need buffered readers/writers here. Probably will need
        // to use Content-Length header for messages similar to what LSP uses.

        stream.write_all(b"hello\nworld!").await.unwrap();
        stream.close().await.unwrap();
        Ok(())
    }
}
