use crate::error::*;
use mio;
use signal_hook;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

#[derive(Debug)]
pub struct SignalEvent {
    pub signal_flag: Arc<AtomicBool>,
    registration: mio::Registration,
}

pub fn listen_on_exit() -> Result<SignalEvent, Error> {
    let (registration, set_register) = mio::Registration::new2();
    let flag = Arc::new(AtomicBool::new(false));

    let event = SignalEvent {
        signal_flag: flag.clone(),
        registration,
    };

    let signals = signal_hook::iterator::Signals::new(&[signal_hook::SIGINT, signal_hook::SIGTERM])
        .map_err(|e| IoError::new("registering signals", e))?;

    thread::Builder::new()
        .name("Signal Listener".to_owned())
        .spawn(move || {
            for _signal in signals.forever() {
                flag.store(true, Ordering::SeqCst);
                let err = set_register.set_readiness(mio::Ready::readable());

                if err.is_err() {
                    eprintln!(
                        "Error sending Exit signal via mio::Poll user event: {:?}",
                        err
                    );
                }
            }
        })
        .map_err(|e| IoError::new("spawning signal listening thread", e))?;

    Ok(event)
}

impl SignalEvent {
    pub fn has_signal(&self) -> bool {
        self.signal_flag.load(Ordering::SeqCst)
    }
}
impl mio::Evented for SignalEvent {
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
