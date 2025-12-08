use crate::watch::watch;
use idle::{IdleTracker, track_idle_time};
use request::{Cache, PfIn, RtStIn};
use smol::{
    Timer,
    future::FutureExt,
    io,
    lock::Mutex,
    net::{TcpListener, TcpStream},
    stream::StreamExt,
};
use std::{sync::Arc, time::Duration};

pub struct Server {
    pub idle_tracker: Arc<IdleTracker>,
}

impl Server {
    pub async fn new(max_idle_time: Duration) -> Self {
        let idle_tracker = Arc::new(IdleTracker::new(max_idle_time));
        smol::spawn(track_idle_time(idle_tracker.clone())).detach();
        Self { idle_tracker }
    }
}

pub fn server_main(max_idle_time: Duration) -> io::Result<()> {
    smol::block_on(async {
        let listener = TcpListener::bind("127.0.0.1:6000").await?;
        println!("Listening on {}", listener.local_addr()?);

        let mut incoming = listener.incoming();
        let server = Arc::new(Server::new(max_idle_time).await);
        let rt = Arc::new(Mutex::new(RtStIn::<PfIn>::new(
            Cache::load("adept.cache"),
            Some(server.idle_tracker.clone()),
        )));

        smol::spawn(watch(server.clone(), rt.clone(), request::ListSymbols)).detach();

        loop {
            let mut next_connection = async || Ok(incoming.next().await);
            let timeout = async || Err(Timer::after(Duration::from_millis(100)).await);

            if let Ok(Some(Ok(stream))) = next_connection().or(timeout()).await {
                smol::spawn(talk_to_client(server.clone(), stream)).detach();
            }

            if server.idle_tracker.shutting_down() {
                break;
            }
        }

        // Try to save the cache to disk before we exit if possible
        if rt.lock().await.cache_to_disk {
            let _ = rt.lock().await.cache().save("adept.cache");
        }

        Ok(())
    })
}

async fn talk_to_client(server: Arc<Server>, stream: TcpStream) -> io::Result<()> {
    if server.idle_tracker.add_connection().is_ok() {
        let _ = server.serve(stream).await;
        server.idle_tracker.remove_connection();
    }
    Ok(())
}
