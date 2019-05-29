extern crate signal_hook;

use super::error::*;
use channel;
use std::thread;

pub enum SignalType {
    Exit,
}

pub fn listen_on_exit() -> Result<channel::Receiver<SignalType>, Error> {
    let (s, r) = channel::unbounded();

    let signals = signal_hook::iterator::Signals::new(&[signal_hook::SIGINT, signal_hook::SIGTERM])
        .map_err(|e| IoError::new("registering signals", e))?;

    thread::Builder::new()
        .name("Signal Listener".to_owned())
        .spawn(move || {
            for _signal in signals.forever() {
                let err = s.send(SignalType::Exit);
                if err.is_err() {
                    eprintln!("Error sending Exit signal over channel: {:?}", err);
                }
            }
        })
        .map_err(|e| IoError::new("spawning signal listening thread", e))?;

    Ok(r)
}
