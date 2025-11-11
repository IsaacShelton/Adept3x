use smol::{
    io::{self, AsyncWriteExt},
    net::TcpStream,
};

pub async fn serve(mut stream: TcpStream) -> io::Result<()> {
    println!("Server received connection");
    stream.write_all(b"hello\nworld!").await.unwrap();
    stream.close().await.unwrap();
    Ok(())
}
