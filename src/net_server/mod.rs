use crate::error::{Error, IoError};
use crate::exit_listener;
use mio;
use mio::net::UdpSocket;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct NetServer {
    que: Arc<Mutex<SendQueue>>,
    ned_data_readiness: mio::SetReadiness,
}

const UDP_TOKEN: mio::Token = mio::Token(0);
const EXIT_TOKEN: mio::Token = mio::Token(1);
const SEND_DATA_TOKEN: mio::Token = mio::Token(2);

impl NetServer {
    pub fn new(addr: SocketAddr, stopper: exit_listener::SignalEvent) -> Result<Self, Error> {
        let socket = UdpSocket::bind(&addr).map_err(|e| IoError::new("creating a socket", e))?;

        let poll = mio::Poll::new().map_err(|e| IoError::new("creating mio::Poll", e))?;

        poll.register(
            &socket,
            UDP_TOKEN,
            mio::Ready::readable(),
            mio::PollOpt::level(),
        )
        .map_err(|e| IoError::new(format!("Registering UdpSocket {} to poll", addr), e))?;

        poll.register(
            &stopper,
            EXIT_TOKEN,
            mio::Ready::readable(),
            mio::PollOpt::edge(),
        )
        .map_err(|e| IoError::new("Registering SignalEvent to poll", e))?;

        let (registration, set_readiness) = mio::Registration::new2();
        let que = Arc::new(Mutex::new(SendQueue::new(registration)));

        poll.register(
            &*que.lock().unwrap(),
            SEND_DATA_TOKEN,
            mio::Ready::readable(),
            mio::PollOpt::edge(),
        )
        .map_err(|e| IoError::new("Registering SendQueue to poll", e))?;

        let res = Self {
            que: que.clone(),
            ned_data_readiness: set_readiness,
        };

        let poll_loop = PollLoop {
            poll,
            socket,
            stopper,
            clients: Vec::new(),
            que,
        };

        thread::spawn(move || poll_loop.poll_loop());

        Ok(res)
    }

    pub fn send_to_all(&self, buf: &[u8]) -> Result<(), Error> {
        if buf.is_empty() {
            return Ok(());
        }

        let mut que = self.que.lock().unwrap();
        let mut block = match que.free.pop() {
            Some(block) => block,
            None => Vec::new(),
        };

        block.extend_from_slice(buf);
        que.to_send.push_back(block);

        self.ned_data_readiness
            .set_readiness(mio::Ready::readable())
            .map_err(|e| IoError::new("sending signal to Poll of a new data block", e))?;

        Ok(())
    }
}

struct PollLoop {
    poll: mio::Poll,
    socket: UdpSocket,
    stopper: exit_listener::SignalEvent,
    clients: Vec<SocketAddr>,
    que: Arc<Mutex<SendQueue>>,
}

struct SendQueue {
    to_send: VecDeque<Vec<u8>>,
    free: Vec<Vec<u8>>,
    registration: mio::Registration,
}

impl PollLoop {
    fn poll_loop(mut self) {
        let mut buf = vec![0; 1024];
        let mut events = mio::Events::with_capacity(1024);
        loop {
            self.poll.poll(&mut events, None).unwrap();
            for event in &events {
                match event.token() {
                    UDP_TOKEN => {
                        let res = self.socket.recv_from(buf.as_mut_slice());

                        match res {
                            Ok((n, back_addr)) => {
                                self.new_connection(&buf[..n], back_addr);
                            }
                            Err(e) => self.read_err(e),
                        };
                    }
                    EXIT_TOKEN => {
                        if self.stopper.has_signal() {
                            return;
                        }
                    }
                    SEND_DATA_TOKEN => self.send_new_data(),
                    _ => {}
                }
            }
        }
    }

    fn new_connection(&mut self, buf: &[u8], addr: SocketAddr) {
        match buf {
            b"info" => self.send_info(&addr),
            b"start" => self.add_new_client(addr),
            b"stop" => self.remove_client(&addr),
            _ => {
                eprintln!("Unknown request: {:?}", buf);
            }
        }
    }

    fn send_new_data(&mut self) {
        let mut que = self.que.lock().unwrap();

        while let Some(mut block) = que.to_send.pop_front() {
            let mut clients_to_remove = Vec::new();

            for (idx, addr) in self.clients.iter().enumerate() {
                let res = self.socket.send_to(&block, &addr);
                if let Err(e) = res {
                    eprintln!("Error sending data block to {}. {}", addr, e);
                    clients_to_remove.push(idx);
                }
            }

            for to_remove in clients_to_remove {
                self.clients.remove(to_remove);
            }

            block.clear();
            que.free.push(block);
        }
    }

    fn send_info(&self, addr: &SocketAddr) {
        let res = self.socket.send_to(b"Hi, how are you?", &addr);
        if let Err(e) = res {
            eprintln!("Error sending: info to {}. {}", addr, e);
        }
    }

    fn add_new_client(&mut self, addr: SocketAddr) {
        let idx = self.clients.iter().position(|&r| r == addr);
        match idx {
            Some(_) => {
                eprintln!("Client {} is already listening", addr);
            }
            None => {
                eprintln!("New client listening: {}", addr);
                self.clients.push(addr);
            }
        }
    }

    fn remove_client(&mut self, addr: &SocketAddr) {
        eprintln!("{} client disconnected", addr);
        self.clients.retain(|&r| r != *addr);
    }

    fn read_err(&self, e: std::io::Error) {
        match e.kind() {
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => (),
            _ => {
                println!("Error receiving: {}", e);
            }
        }
    }
}

impl SendQueue {
    fn new(registration: mio::Registration) -> Self {
        Self {
            to_send: VecDeque::new(),
            free: Vec::new(),
            registration,
        }
    }
}
impl mio::Evented for SendQueue {
    fn register(
        &self,
        poll: &mio::Poll,
        token: mio::Token,
        interest: mio::Ready,
        opts: mio::PollOpt,
    ) -> Result<(), std::io::Error> {
        self.registration.register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &mio::Poll,
        token: mio::Token,
        interest: mio::Ready,
        opts: mio::PollOpt,
    ) -> Result<(), std::io::Error> {
        self.registration.reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> Result<(), std::io::Error> {
        poll.deregister(&self.registration)
    }
}
