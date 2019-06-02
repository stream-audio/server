use audio_sharing_pc::alsa;
use audio_sharing_pc::error::*;
use audio_sharing_pc::run;

fn main() -> Result<(), Error> {
    alsa::list_devices()?;

    let params = alsa::Params {
        format: alsa::Format::S16Le,
        channels: 2,
        rate: 44100,
    };

    alsa::record("hw:3,1".to_owned(), params)?;

    /*
    match run() {
        Ok(_) => {}
        Err(err) => println!("{}", err),
    }
    */

    Ok(())
}
