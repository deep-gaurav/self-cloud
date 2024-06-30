use rand::distributions::{Alphanumeric, DistString};

pub mod xterm;

pub fn random_ascii_string(length: usize) -> String {
    let chars: String = Alphanumeric.sample_string(&mut rand::thread_rng(), length);
    chars
}
