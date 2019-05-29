extern crate audio_sharing_pc;

use audio_sharing_pc::run;

fn main() {
    match run() {
        Ok(_) => {}
        Err(err) => println!("{}", err),
    }
}
