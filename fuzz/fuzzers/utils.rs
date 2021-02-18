extern crate base64;
extern crate rand;
extern crate rand_pcg;
extern crate ring;

use base64::{alphabet, engine::fast_portable};
use self::rand::{Rng, SeedableRng};
use self::rand_pcg::Pcg32;
use self::ring::digest;

pub fn random_engine(data: &[u8]) -> fast_portable::FastPortable {
    // use sha256 of data as rng seed so it's repeatable
    let sha = digest::digest(&digest::SHA256, data);

    let mut seed: [u8; 16] = [0; 16];
    seed.copy_from_slice(&sha.as_ref()[0..16]);

    let mut rng = Pcg32::from_seed(seed);

    let alphabet = if rng.gen() {
        alphabet::URL_SAFE
    } else {
        alphabet::STANDARD
    };

    let config = fast_portable::FastPortableConfig::from(rng.gen(), rng.gen());

    fast_portable::FastPortable::from(&alphabet, config)
}
