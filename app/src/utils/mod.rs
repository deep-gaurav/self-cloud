use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng,
};

pub fn random_ascii_string(length: usize) -> String {
    let rng = thread_rng();
    let chars: String = Alphanumeric.sample_string(&mut rand::thread_rng(), length);
    chars
}
