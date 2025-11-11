use crate::{
    fs_watcher::FsWatcher,
    idle::{IdleTracker, track_idle_time},
    serve::serve,
};
use smol::{
    Timer,
    future::FutureExt,
    io,
    lock::Mutex,
    net::{TcpListener, TcpStream},
    stream::StreamExt,
};
use std::{
    sync::{Arc, mpsc},
    time::Duration,
};

#[derive(Debug)]
pub enum Event {}

pub fn server_main() -> io::Result<()> {
    let (_rx, _tx) = mpsc::channel::<Event>();

    smol::block_on(async {
        // Create a listener.
        let listener = TcpListener::bind("127.0.0.1:6000").await?;
        println!("Listening on {}", listener.local_addr()?);
        let mut incoming = listener.incoming();

        let idle_tracker = Arc::new(Mutex::new(IdleTracker::new()));
        smol::spawn(track_idle_time(idle_tracker.clone())).detach();

        let watcher = Arc::new(FsWatcher::new());
        smol::spawn(watcher.watch(idle_tracker.clone())).detach();

        loop {
            let mut next_connection = async || Ok(incoming.next().await);
            let timeout = async || Err(Timer::after(Duration::from_millis(100)).await);

            if let Ok(Some(Ok(stream))) = next_connection().or(timeout()).await {
                smol::spawn(talk_to_client(stream, idle_tracker.clone())).detach();
            }

            if idle_tracker.lock().await.shutting_down() {
                break;
            }
        }

        Ok(())
    })
}

async fn talk_to_client(
    stream: TcpStream,
    idle_tracker: Arc<Mutex<IdleTracker>>,
) -> io::Result<()> {
    if idle_tracker.lock().await.add_connection().is_ok() {
        let _ = serve(stream).await;
        idle_tracker.lock().await.remove_connection();
    }
    Ok(())
}
