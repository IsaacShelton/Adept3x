use crate::LspMessage;
use std::{
    io::{self, BufReader, Write},
    sync::mpsc,
    thread::JoinHandle,
};

pub struct LspConnection {
    stdin_join_handle: JoinHandle<io::Result<()>>,
    stdout_join_handle: JoinHandle<io::Result<()>>,
    stdout_tx: mpsc::Sender<LspMessage>,
    stdin_rx: mpsc::Receiver<LspMessage>,
    pub connection_state: LspConnectionState,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LspConnectionState {
    Started,
    Initialized,
    Shutdown,
}

impl LspConnection {
    pub fn wait_for_message(&mut self) -> Option<LspMessage> {
        self.stdin_rx.recv().ok()
    }

    pub fn stdio() -> Self {
        let (stdin_tx, stdin_rx) = mpsc::channel::<LspMessage>();

        let stdin_join_handle = std::thread::spawn(move || {
            let mut stdin = BufReader::new(std::io::stdin().lock());

            loop {
                let Some(message) = LspMessage::read(&mut stdin)? else {
                    continue;
                };

                if let LspMessage::Notification(notification) = &message {
                    if notification.method == "exit" {
                        break;
                    }
                }

                if stdin_tx.send(message).is_err() {
                    log::info!("Nothing left to receive from stdin thread, the channel is closed");
                    break;
                }
            }

            Ok(())
        });

        let (stdout_tx, stdout_rx) = mpsc::channel::<LspMessage>();

        let stdout_join_handle = std::thread::spawn(move || {
            let mut stdout = std::io::stdout().lock();

            loop {
                let Ok(message) = stdout_rx.recv() else {
                    log::info!("Nothing left to send on stdout thread, the channel is closed");
                    break;
                };

                message.write(&mut stdout)?;
                stdout.flush()?;
            }

            Ok(())
        });

        Self {
            stdin_join_handle,
            stdout_join_handle,
            stdout_tx,
            stdin_rx,
            connection_state: LspConnectionState::Started,
        }
    }

    pub fn send(&self, message: LspMessage) {
        match self.stdout_tx.send(message) {
            Ok(()) => (),
            Err(_) => log::info!("Failed to send message to output channel, it's closed."),
        }
    }

    pub fn join(self) {
        join_io_thread("Stdout", self.stdout_join_handle);
        join_io_thread("Stdin", self.stdin_join_handle);
    }
}

fn join_io_thread(title_name: &str, handle: JoinHandle<io::Result<()>>) {
    match handle.join() {
        Ok(Ok(())) => (),
        Ok(Err(io_error)) => log::error!("{} IO Error - {:?}", title_name, io_error),
        Err(join_error) => log::error!("{} IO Thread Join Error - {:?}", title_name, join_error),
    }
}
