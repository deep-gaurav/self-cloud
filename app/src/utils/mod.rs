use rand::distr::{Alphanumeric, SampleString};

pub mod xterm;

pub fn random_ascii_string(length: usize) -> String {
    let chars: String = Alphanumeric.sample_string(&mut rand::rng(), length);
    chars
}
