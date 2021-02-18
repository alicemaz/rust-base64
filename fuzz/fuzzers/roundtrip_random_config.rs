#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate base64;

use base64::*;

mod utils;

fuzz_target!(|data: &[u8]| {
    let engine = utils::random_engine(data);

    let encoded = encode_engine(&data, &engine);
    let decoded = decode_engine(&encoded, &engine).unwrap();
    assert_eq!(data, decoded.as_slice());
});
