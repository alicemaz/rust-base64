extern crate base64;
extern crate rand;

use rand::{FromEntropy, Rng};

use base64::engine::{Engine, DEFAULT_ENGINE};
use base64::*;

use self::helpers::*;
use base64::alphabet::STANDARD;
use base64::engine::fast_portable::{FastPortable, NO_PAD};

mod helpers;

// generate random contents of the specified length and test encode/decode roundtrip
fn roundtrip_random<E: Engine>(
    byte_buf: &mut Vec<u8>,
    str_buf: &mut String,
    engine: &E,
    byte_len: usize,
    approx_values_per_byte: u8,
    max_rounds: u64,
) {
    // let the short ones be short but don't let it get too crazy large
    let num_rounds = calculate_number_of_rounds(byte_len, approx_values_per_byte, max_rounds);
    let mut r = rand::rngs::SmallRng::from_entropy();
    let mut decode_buf = Vec::new();

    for _ in 0..num_rounds {
        byte_buf.clear();
        str_buf.clear();
        decode_buf.clear();
        while byte_buf.len() < byte_len {
            byte_buf.push(r.gen::<u8>());
        }

        encode_engine_string(&byte_buf, str_buf, engine);
        decode_engine_vec(&str_buf, &mut decode_buf, engine).unwrap();

        assert_eq!(byte_buf, &decode_buf);
    }
}

fn calculate_number_of_rounds(byte_len: usize, approx_values_per_byte: u8, max: u64) -> u64 {
    // don't overflow
    let mut prod = approx_values_per_byte as u64;

    for _ in 0..byte_len {
        if prod > max {
            return max;
        }

        prod = prod.saturating_mul(prod);
    }

    prod
}

#[test]
fn roundtrip_random_short_standard() {
    let mut byte_buf: Vec<u8> = Vec::new();
    let mut str_buf = String::new();

    for input_len in 0..40 {
        roundtrip_random(
            &mut byte_buf,
            &mut str_buf,
            &DEFAULT_ENGINE,
            input_len,
            4,
            10000,
        );
    }
}

#[test]
fn roundtrip_random_with_fast_loop_standard() {
    let mut byte_buf: Vec<u8> = Vec::new();
    let mut str_buf = String::new();

    for input_len in 40..100 {
        roundtrip_random(
            &mut byte_buf,
            &mut str_buf,
            &DEFAULT_ENGINE,
            input_len,
            4,
            1000,
        );
    }
}

#[test]
fn roundtrip_random_short_no_padding() {
    let mut byte_buf: Vec<u8> = Vec::new();
    let mut str_buf = String::new();

    let engine = FastPortable::from(&STANDARD, NO_PAD);
    for input_len in 0..40 {
        roundtrip_random(&mut byte_buf, &mut str_buf, &engine, input_len, 4, 10000);
    }
}

#[test]
fn roundtrip_random_no_padding() {
    let mut byte_buf: Vec<u8> = Vec::new();
    let mut str_buf = String::new();

    let engine = FastPortable::from(&STANDARD, NO_PAD);

    for input_len in 40..100 {
        roundtrip_random(&mut byte_buf, &mut str_buf, &engine, input_len, 4, 1000);
    }
}

#[test]
fn roundtrip_decode_trailing_10_bytes() {
    // This is a special case because we decode 8 byte blocks of input at a time as much as we can,
    // ideally unrolled to 32 bytes at a time, in stages 1 and 2. Since we also write a u64's worth
    // of bytes (8) to the output, we always write 2 garbage bytes that then will be overwritten by
    // the NEXT block. However, if the next block only contains 2 bytes, it will decode to 1 byte,
    // and therefore be too short to cover up the trailing 2 garbage bytes. Thus, we have stage 3
    // to handle that case.

    for num_quads in 0..25 {
        let mut s: String = std::iter::repeat("ABCD").take(num_quads).collect();
        s.push_str("EFGHIJKLZg");

        let decoded = decode(&s).unwrap();
        assert_eq!(num_quads * 3 + 7, decoded.len());

        assert_eq!(
            s,
            encode_engine(&decoded, &FastPortable::from(&STANDARD, NO_PAD))
        );
    }
}

#[test]
fn display_wrapper_matches_normal_encode() {
    let mut bytes = Vec::<u8>::with_capacity(256);

    for i in 0..255 {
        bytes.push(i);
    }
    bytes.push(255);

    assert_eq!(
        encode(&bytes),
        format!(
            "{}",
            base64::display::Base64Display::from(&bytes, &DEFAULT_ENGINE)
        )
    );
}

#[test]
fn because_we_can() {
    compare_decode("alice", "YWxpY2U=");
    compare_decode("alice", &encode(b"alice"));
    compare_decode("alice", &encode(&decode(&encode(b"alice")).unwrap()));
}

#[test]
fn encode_engine_slice_can_use_inline_buffer() {
    let mut buf: [u8; 22] = [0; 22];
    let mut larger_buf: [u8; 24] = [0; 24];
    let mut input: [u8; 16] = [0; 16];

    let engine = FastPortable::from(&STANDARD, NO_PAD);

    let mut rng = rand::rngs::SmallRng::from_entropy();
    for elt in &mut input {
        *elt = rng.gen();
    }

    assert_eq!(22, encode_engine_slice(&input, &mut buf, &engine));
    let decoded = decode_engine(&buf, &engine).unwrap();

    assert_eq!(decoded, input);

    // let's try it again with padding

    assert_eq!(
        24,
        encode_engine_slice(&input, &mut larger_buf, &DEFAULT_ENGINE)
    );
    let decoded = decode_engine(&buf, &DEFAULT_ENGINE).unwrap();

    assert_eq!(decoded, input);
}

#[test]
#[should_panic(expected = "index 24 out of range for slice of length 22")]
fn encode_engine_slice_panics_when_buffer_too_small() {
    let mut buf: [u8; 22] = [0; 22];
    let mut input: [u8; 16] = [0; 16];

    let mut rng = rand::rngs::SmallRng::from_entropy();
    for elt in &mut input {
        *elt = rng.gen();
    }

    encode_engine_slice(&input, &mut buf, &DEFAULT_ENGINE);
}
