pub mod receive {
    use std::{thread, sync::{mpsc, Arc, atomic::{AtomicBool, Ordering}}, time::Duration, env, path::Path, os::unix::net::{UnixListener, UnixStream}, io::{Read, self, Write}, fs};
    use serde::Deserialize;
    pub struct SocketListener {
        thread: Option<thread::JoinHandle<()>>
    }

    #[derive(Debug, Clone)]
    pub enum SocketListenerCreationError {
        /// The `HOME` environment variable is not defined.
        NoRuntimeDir,
        /// An error occured while trying to create the socket.
        UnableToCreateSocket,
        /// An error occured while trying to set the socket to non-blocking.
        UnableToSetNonBlocking
    }

    #[derive(Deserialize, Debug)]
    #[serde(tag = "type")]
    pub enum SocketListenerCommand {
        /// IP Address, port, encryption key, and PID to capture from
        StartStream { ip: String, port: u16, key: Vec<u8>, pid: usize },
        StopStream,
        GetInfo
    }

    impl SocketListener {
        pub fn new(sender: mpsc::Sender<SocketListenerCommand>, run: Arc<AtomicBool>, sleep_time: Duration) -> Result<SocketListener, SocketListenerCreationError> {
            // Attempt to load env var
            let key = match env::var("HOME") {
                Ok(val) => val,
                Err(_) => return Err(SocketListenerCreationError::NoRuntimeDir)
            };
        
            let path = Path::new(&key).join(".config").join("tuxphones.sock");

            let listener = match UnixListener::bind(&path) {
                Ok(sock) => sock,
                Err(e) => {
                    eprintln!("Failed to create listener: {}", e);
                    return Err(SocketListenerCreationError::UnableToCreateSocket)
                }
            };

            // Allows for constant event processing
            match listener.set_nonblocking(true) {
                Err(e) => {
                    eprintln!("Failed to set listener to non-blocking: {}", e);
                    return Err(SocketListenerCreationError::UnableToSetNonBlocking);
                }
                Ok(()) => {}
            }

            // Spawn listener thread to check for commands sent to the socket
            let thread = thread::spawn(move || {
                for stream in listener.incoming() {
                    match stream {
                        Ok(mut stream) => {
                            let mut buf = String::new();
                            match stream.read_to_string(&mut buf) {
                                Ok(_) => {}
                                Err(e) => {
                                    eprintln!("Failed to read socket stream: {}", e);
                                    continue;
                                }
                            }

                            println!("Command received: {}", buf);

                            match serde_json::from_str::<SocketListenerCommand>(&buf) {
                                Ok(cmd) => match sender.send(cmd) {
                                    Ok(_) => {},
                                    Err(e) => eprintln!("Failed to send command: {}", e)
                                },
                                Err(e) => eprintln!("Failed to deserialize command: {}", e),
                            }
                        },
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            if !run.load(Ordering::SeqCst) {
                                match fs::remove_file(&path) {
                                    Ok(()) => println!("Cleaned up socket"),
                                    Err(e) => eprintln!("Failed to remove socket: {}", e)
                                }
                                break;
                            }
                            thread::sleep(sleep_time);
                        }
                        Err(e) => eprintln!("Failed to get stream: {}", e)
                    }
                }
            });

            Ok(SocketListener { thread: Some(thread) })
        }

        /// Waits for the `SocketListeners`'s internal thread to join.
        pub fn join(&mut self) {
            if let Some(thread) = self.thread.take() {
                thread.join().expect("Unable to join thread");
            }
        }
    }
}