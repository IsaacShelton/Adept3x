use crate::server::Server;
use smol::{
    io::{self, AsyncWriteExt},
    net::TcpStream,
};

impl Server {
    pub async fn serve(&self, mut stream: TcpStream) -> io::Result<()> {
        println!("Server received connection");
        stream.write_all(b"hello\nworld!").await.unwrap();
        stream.close().await.unwrap();
        Ok(())
    }
}
