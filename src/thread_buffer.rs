use crate::error::*;
use std::cmp;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::thread::JoinHandle;

pub trait DataReceiver: Send {
    fn new_slice(&mut self, data: &[u8]) -> Result<(), Error>;
}

pub struct ThreadBuffer {
    que: Arc<ThreadQueue>,
    thread: Option<JoinHandle<()>>,
}

impl ThreadBuffer {
    pub fn new(writer: Box<dyn DataReceiver>) -> Self {
        let que = Arc::new(ThreadQueue::default());
        let thread_self = ThreadData::new(que.clone(), writer);

        let thread = thread::Builder::new()
            .name("ThreadBuffer".to_owned())
            .spawn(move || thread_self.thread_loop())
            .expect("Error creating a thread buffer");

        Self {
            que,
            thread: Some(thread),
        }
    }

    pub fn write_data(&self, data: &[u8]) -> Result<(), Error> {
        let mut que = self.que.que.lock().unwrap();
        if let Some(que) = que.as_mut() {
            que.extend(data);
            self.que.cvar.notify_one();
        }
        Ok(())
    }

    /// Is not thread safe, must be called from the thread it was created
    pub fn stop_and_join(&mut self) {
        if let Some(thread) = self.thread.take() {
            *self.que.que.lock().unwrap() = None;
            self.que.cvar.notify_one();
            thread.join().unwrap();
        }
    }
}
impl Drop for ThreadBuffer {
    /// Is not thread safe
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

struct ThreadQueue {
    que: Mutex<Option<VecDeque<u8>>>,
    cvar: Condvar,
}
impl Default for ThreadQueue {
    fn default() -> Self {
        Self {
            que: Mutex::new(Some(VecDeque::with_capacity(4096))),
            cvar: Condvar::new(),
        }
    }
}

const BUFFER_SIZE: usize = 4096;
struct ThreadData {
    que: Arc<ThreadQueue>,
    writer: Box<dyn DataReceiver>,
    buffer: Vec<u8>,
}

impl ThreadData {
    fn new(que: Arc<ThreadQueue>, writer: Box<dyn DataReceiver>) -> Self {
        Self {
            que,
            writer,
            buffer: Vec::with_capacity(BUFFER_SIZE),
        }
    }

    fn thread_loop(mut self) {
        loop {
            let can_continue = self.wait_and_fill_buffer();
            if !can_continue {
                return;
            }

            let write_res = self.writer.new_slice(&self.buffer);
            if let Err(err) = write_res {
                eprintln!("Error writing audio: {}", err);
                return;
            }
            self.buffer.clear();
        }
    }

    fn wait_and_fill_buffer(&mut self) -> bool {
        let mut lock = self.que.que.lock().unwrap();

        let que = loop {
            match *lock {
                Some(ref mut que) => {
                    if !que.is_empty() {
                        break que;
                    }
                }
                None => {
                    return false;
                }
            }

            lock = self.que.cvar.wait(lock).unwrap();
        };

        let drain_len = cmp::min(que.len(), BUFFER_SIZE);
        let drained = que.drain(..drain_len);

        self.buffer.extend(drained.into_iter());

        true
    }
}
