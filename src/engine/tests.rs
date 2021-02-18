// rstest_reuse template functions have unused variables
#![allow(unused_variables)]

use rand::{self, distributions::Distribution, distributions::Uniform, FromEntropy, Rng};
use rstest::rstest;
use rstest_reuse::{apply, template};
use std::iter;

use crate::{
    alphabet::{Alphabet, STANDARD},
    encode,
    engine::{fast_portable, naive, Engine},
    tests::random_alphabet,
    DecodeError, PAD_BYTE,
};

#[template]
#[rstest(engine_wrapper,
case(FastPortableWrapper {}),
case(NaiveWrapper {}),
)]
fn all_engines<E: EngineWrapper>(engine_wrapper: E) {}

#[apply(all_engines)]
fn rfc_test_vectors_std_alphabet<E: EngineWrapper>(engine_wrapper: E) {
    let data = vec![
        ("", ""),
        ("f", "Zg"),
        ("fo", "Zm8"),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg"),
        ("fooba", "Zm9vYmE"),
        ("foobar", "Zm9vYmFy"),
    ];

    let engine = E::standard();

    for (orig, encoded) in &data {
        let mut encode_buf = [0_u8; 8];
        let mut decode_buf = [0_u8; 6];

        let encode_len = engine.encode(orig.as_bytes(), &mut encode_buf[..]);

        assert_eq!(
            encoded,
            &std::str::from_utf8(&encode_buf[0..encode_len]).unwrap()
        );

        // unpadded
        {
            let decode_len = engine
                .decode_ez(&encode_buf[0..encode_len], &mut decode_buf[..])
                .unwrap();
            assert_eq!(orig.len(), decode_len);

            assert_eq!(
                orig,
                &std::str::from_utf8(&decode_buf[0..decode_len]).unwrap()
            );
        }

        // padded
        {
            let _ = encode::add_padding(orig.len(), &mut encode_buf[encode_len..]);

            let decode_len = engine
                .decode_ez(&encode_buf[0..encode_len], &mut decode_buf[..])
                .unwrap();
            assert_eq!(orig.len(), decode_len);

            assert_eq!(
                orig,
                &std::str::from_utf8(&decode_buf[0..decode_len]).unwrap()
            );
        }
    }
}

#[apply(all_engines)]
fn roundtrip_random<E: EngineWrapper>(engine_wrapper: E) {
    let mut rng = rand::rngs::SmallRng::from_entropy();

    let mut orig_data = Vec::<u8>::new();
    let mut encode_buf = Vec::<u8>::new();
    let mut decode_buf = Vec::<u8>::new();

    let len_range = Uniform::new(1, 1_000);

    for _ in 0..10_000 {
        let engine = E::random(&mut rng);

        orig_data.clear();
        encode_buf.clear();
        decode_buf.clear();

        let (orig_len, _, encoded_len) = generate_random_encoded_data(
            &engine,
            &mut orig_data,
            &mut encode_buf,
            &mut rng,
            &len_range,
        );

        // exactly the right size
        decode_buf.resize(orig_len, 0);

        let dec_len = engine
            .decode_ez(&encode_buf[0..encoded_len], &mut decode_buf[..])
            .unwrap();

        assert_eq!(orig_len, dec_len);
        assert_eq!(&orig_data[..], &decode_buf[..dec_len]);
    }
}

#[apply(all_engines)]
fn encode_doesnt_write_extra_bytes<E: EngineWrapper>(engine_wrapper: E) {
    let mut rng = rand::rngs::SmallRng::from_entropy();

    let mut orig_data = Vec::<u8>::new();
    let mut encode_buf = Vec::<u8>::new();
    let mut encode_buf_backup = Vec::<u8>::new();

    let len_range = Uniform::new(1, 1_000);

    for _ in 0..10_000 {
        let engine = E::random(&mut rng);

        orig_data.clear();
        encode_buf.clear();
        encode_buf_backup.clear();

        let orig_len = fill_rand(&mut orig_data, &mut rng, &len_range);
        let expected_encode_len = engine_encoded_len(orig_len);
        encode_buf.resize(expected_encode_len, 0);

        // oversize encode buffer so we can easily tell if it writes anything more than
        // just the encoded data
        fill_rand_len(&mut encode_buf, &mut rng, (expected_encode_len + 100) * 2);
        encode_buf_backup.extend_from_slice(&encode_buf[..]);

        let encoded_len = engine.encode(&orig_data[..], &mut encode_buf[..]);
        assert_eq!(expected_encode_len, encoded_len);

        // no writes past what it claimed to write
        assert_eq!(
            &encode_buf_backup[encoded_len..],
            &encode_buf[encoded_len..]
        )
    }
}

#[apply(all_engines)]
fn decode_doesnt_write_extra_bytes<E: EngineWrapper>(engine_wrapper: E) {
    let mut rng = rand::rngs::SmallRng::from_entropy();

    let mut orig_data = Vec::<u8>::new();
    let mut encode_buf = Vec::<u8>::new();
    let mut decode_buf = Vec::<u8>::new();
    let mut decode_buf_backup = Vec::<u8>::new();

    let len_range = Uniform::new(1, 1_000);

    for _ in 0..10_000 {
        let engine = E::random(&mut rng);

        orig_data.clear();
        encode_buf.clear();
        decode_buf.clear();
        decode_buf_backup.clear();

        let orig_len = fill_rand(&mut orig_data, &mut rng, &len_range);
        encode_buf.resize(engine_encoded_len(orig_len), 0);

        let encoded_len = engine.encode(&orig_data[..], &mut encode_buf[..]);

        // oversize decode buffer so we can easily tell if it writes anything more than
        // just the decoded data
        fill_rand_len(&mut decode_buf, &mut rng, (orig_len + 100) * 2);
        decode_buf_backup.extend_from_slice(&decode_buf[..]);

        let dec_len = engine
            .decode_ez(&encode_buf[0..encoded_len], &mut decode_buf[..])
            .unwrap();

        assert_eq!(orig_len, dec_len);
        assert_eq!(&orig_data[..], &decode_buf[..dec_len]);
        assert_eq!(&decode_buf_backup[dec_len..], &decode_buf[dec_len..]);
    }
}

#[apply(all_engines)]
fn decode_detect_invalid_last_symbol_one_byte<E: EngineWrapper>(engine_wrapper: E) {
    // 0xFF -> "/w==", so all letters > w, 0-9, and '+', '/' should get InvalidLastSymbol
    let engine = E::standard();

    assert_eq!(Ok(vec![0xFF]), engine.decode_ez_str_vec("/w=="));
    assert_eq!(
        Err(DecodeError::InvalidLastSymbol(1, b'x')),
        engine.decode_ez_str_vec("/x==")
    );
    assert_eq!(
        Err(DecodeError::InvalidLastSymbol(1, b'z')),
        engine.decode_ez_str_vec("/z==")
    );
    assert_eq!(
        Err(DecodeError::InvalidLastSymbol(1, b'0')),
        engine.decode_ez_str_vec("/0==")
    );
    assert_eq!(
        Err(DecodeError::InvalidLastSymbol(1, b'9')),
        engine.decode_ez_str_vec("/9==")
    );
    assert_eq!(
        Err(DecodeError::InvalidLastSymbol(1, b'+')),
        engine.decode_ez_str_vec("/+==")
    );
    assert_eq!(
        Err(DecodeError::InvalidLastSymbol(1, b'/')),
        engine.decode_ez_str_vec("//==")
    );

    // also works when it's not the only chunk
    let mut prefix = String::new();
    for _ in 0..50 {
        prefix.push_str("AAAA");

        let mut input = prefix.clone();
        input.push_str("/x==");

        assert_eq!(
            Err(DecodeError::InvalidLastSymbol(prefix.len() + 1, b'x')),
            engine.decode_ez_str_vec(input.as_str())
        );
    }
}

#[apply(all_engines)]
fn decode_detect_invalid_last_symbol_two_bytes<E: EngineWrapper>(engine_wrapper: E) {
    let engine = E::standard();

    // example from https://github.com/marshallpierce/rust-base64/issues/75
    assert!(engine.decode_ez_str_vec("iYU=").is_ok());
    // trailing 01
    assert_eq!(
        Err(DecodeError::InvalidLastSymbol(2, b'V')),
        engine.decode_ez_str_vec("iYV=")
    );
    // trailing 10
    assert_eq!(
        Err(DecodeError::InvalidLastSymbol(2, b'W')),
        engine.decode_ez_str_vec("iYW=")
    );
    // trailing 11
    assert_eq!(
        Err(DecodeError::InvalidLastSymbol(2, b'X')),
        engine.decode_ez_str_vec("iYX=")
    );

    // also works when it's not the only chunk
    let mut prefix = String::new();
    for _ in 0..50 {
        prefix.push_str("AAAA");

        let mut input = prefix.clone();
        input.push_str("iYX=");

        assert_eq!(
            Err(DecodeError::InvalidLastSymbol(prefix.len() + 2, b'X')),
            engine.decode_ez_str_vec(input.as_str())
        );
    }
}

#[apply(all_engines)]
fn decode_detect_invalid_last_symbol_every_possible_two_symbols<E: EngineWrapper>(
    engine_wrapper: E,
) {
    let engine = E::standard();

    let mut base64_to_bytes = ::std::collections::HashMap::new();

    for b in 0_u16..256 {
        let mut b64 = vec![0_u8; 4];
        assert_eq!(2, engine.encode(&[b as u8], &mut b64[..]));
        let _ = encode::add_padding(1, &mut b64[2..]);

        assert!(base64_to_bytes.insert(b64, vec![b as u8]).is_none());
    }

    // every possible combination of trailing symbols must either decode to 1 byte or get InvalidLastSymbol, with or without any leading chunks

    let mut prefix = Vec::new();
    for _ in 0..50 {
        let mut clone = prefix.clone();

        let mut symbols = [0_u8; 4];
        for &s1 in STANDARD.symbols.iter() {
            symbols[0] = s1;
            for &s2 in STANDARD.symbols.iter() {
                symbols[1] = s2;
                symbols[2] = PAD_BYTE;
                symbols[3] = PAD_BYTE;

                // chop off previous symbols
                clone.truncate(prefix.len());
                clone.extend_from_slice(&symbols[..]);
                let decoded_prefix_len = prefix.len() / 4 * 3;

                match base64_to_bytes.get(&symbols[..]) {
                    Some(bytes) => {
                        let res = engine
                            .decode_ez_vec(&clone)
                            // remove prefix
                            .map(|decoded| decoded[decoded_prefix_len..].to_vec());

                        assert_eq!(Ok(bytes.clone()), res)
                    }
                    None => assert_eq!(
                        Err(DecodeError::InvalidLastSymbol(1, s2)),
                        engine.decode_ez_vec(&symbols[..])
                    ),
                }
            }
        }

        prefix.extend_from_slice("AAAA".as_bytes());
    }
}

#[apply(all_engines)]
fn decode_detect_invalid_last_symbol_every_possible_three_symbols<E: EngineWrapper>(
    engine_wrapper: E,
) {
    let engine = E::standard();

    let mut base64_to_bytes = ::std::collections::HashMap::new();

    let mut bytes = [0_u8; 2];
    for b1 in 0_u16..256 {
        bytes[0] = b1 as u8;
        for b2 in 0_u16..256 {
            bytes[1] = b2 as u8;
            let mut b64 = vec![0_u8; 4];
            assert_eq!(3, engine.encode(&bytes, &mut b64[..]));
            let _ = encode::add_padding(2, &mut b64[3..]);

            let mut v = ::std::vec::Vec::with_capacity(2);
            v.extend_from_slice(&bytes[..]);

            assert!(base64_to_bytes.insert(b64, v).is_none());
        }
    }

    // every possible combination of symbols must either decode to 2 bytes or get InvalidLastSymbol, with or without any leading chunks

    let mut prefix = Vec::new();
    for _ in 0..50 {
        let mut input = prefix.clone();

        let mut symbols = [0_u8; 4];
        for &s1 in STANDARD.symbols.iter() {
            symbols[0] = s1;
            for &s2 in STANDARD.symbols.iter() {
                symbols[1] = s2;
                for &s3 in STANDARD.symbols.iter() {
                    symbols[2] = s3;
                    symbols[3] = PAD_BYTE;

                    // chop off previous symbols
                    input.truncate(prefix.len());
                    input.extend_from_slice(&symbols[..]);
                    let decoded_prefix_len = prefix.len() / 4 * 3;

                    match base64_to_bytes.get(&symbols[..]) {
                        Some(bytes) => {
                            let res = engine
                                .decode_ez_vec(&input)
                                // remove prefix
                                .map(|decoded| decoded[decoded_prefix_len..].to_vec());

                            assert_eq!(Ok(bytes.clone()), res)
                        }
                        None => assert_eq!(
                            Err(DecodeError::InvalidLastSymbol(2, s3)),
                            engine.decode_ez_vec(&symbols[..])
                        ),
                    }
                }
            }
        }
        prefix.extend_from_slice("AAAA".as_bytes());
    }
}

#[apply(all_engines)]
fn decode_invalid_trailing_bits_ignored_when_configured<E: EngineWrapper>(engine_wrapper: E) {
    let strict = E::standard();
    let forgiving = E::standard_forgiving();

    fn assert_tolerant_decode<E: Engine>(
        engine: &E,
        input: &mut String,
        b64_prefix_len: usize,
        expected_decode_bytes: Vec<u8>,
        data: &str,
    ) {
        let prefixed = prefixed_data(input, b64_prefix_len, data);
        let decoded = engine.decode_ez_str_vec(prefixed);
        // prefix is always complete chunks
        let decoded_prefix_len = b64_prefix_len / 4 * 3;
        assert_eq!(
            Ok(expected_decode_bytes),
            decoded.map(|v| v[decoded_prefix_len..].to_vec())
        );
    }

    let mut prefix = String::new();
    for _ in 0..50 {
        let mut input = prefix.clone();

        // example from https://github.com/marshallpierce/rust-base64/issues/75
        assert!(strict
            .decode_ez_str_vec(prefixed_data(&mut input, prefix.len(), "/w=="))
            .is_ok());
        assert!(strict
            .decode_ez_str_vec(prefixed_data(&mut input, prefix.len(), "iYU="))
            .is_ok());
        // trailing 01
        assert_tolerant_decode(&forgiving, &mut input, prefix.len(), vec![255], "/x==");
        assert_tolerant_decode(&forgiving, &mut input, prefix.len(), vec![137, 133], "iYV=");
        // trailing 10
        assert_tolerant_decode(&forgiving, &mut input, prefix.len(), vec![255], "/y==");
        assert_tolerant_decode(&forgiving, &mut input, prefix.len(), vec![137, 133], "iYW=");
        // trailing 11
        assert_tolerant_decode(&forgiving, &mut input, prefix.len(), vec![255], "/z==");
        assert_tolerant_decode(&forgiving, &mut input, prefix.len(), vec![137, 133], "iYX=");

        prefix.push_str("AAAA");
    }
}

#[apply(all_engines)]
fn decode_invalid_byte_error<E: EngineWrapper>(engine_wrapper: E) {
    let mut rng = rand::rngs::SmallRng::from_entropy();

    let mut orig_data = Vec::<u8>::new();
    let mut encode_buf = Vec::<u8>::new();
    let mut decode_buf = Vec::<u8>::new();

    let len_range = Uniform::new(1, 1_000);

    for _ in 0..10_000 {
        let alphabet = random_alphabet(&mut rng);
        let engine = E::random_alphabet(&mut rng, &alphabet);

        orig_data.clear();
        encode_buf.clear();
        decode_buf.clear();

        let (orig_len, encoded_len_just_data, encoded_len_with_padding) =
            generate_random_encoded_data(
                &engine,
                &mut orig_data,
                &mut encode_buf,
                &mut rng,
                &len_range,
            );

        // exactly the right size
        decode_buf.resize(orig_len, 0);

        // replace one encoded byte with an invalid byte
        let invalid_byte: u8 = loop {
            let byte: u8 = rng.gen();

            if alphabet.symbols.contains(&byte) {
                continue;
            } else {
                break byte;
            }
        };

        let invalid_range = Uniform::new(0, orig_len);
        let invalid_index = invalid_range.sample(&mut rng);
        encode_buf[invalid_index] = invalid_byte;

        assert_eq!(
            Err(DecodeError::InvalidByte(invalid_index, invalid_byte)),
            engine.decode_ez(
                &encode_buf[0..encoded_len_with_padding],
                &mut decode_buf[..],
            )
        );
    }
}

#[apply(all_engines)]
fn decode_single_pad_byte_after_2_chars_in_trailing_quad_ok<E: EngineWrapper>(engine_wrapper: E) {
    let engine = E::standard();

    for num_prefix_quads in 0..50 {
        let mut s: String = iter::repeat("ABCD").take(num_prefix_quads).collect();
        // weird padding, but should be allowed
        s.push_str("Zg=");

        let input_len = num_prefix_quads * 3 + 1;

        assert_eq!(input_len, engine.decode_ez_str_vec(&s).unwrap().len());
    }
}

//this is a MAY in the rfc: https://tools.ietf.org/html/rfc4648#section-3.3
#[apply(all_engines)]
fn decode_pad_byte_in_penultimate_quad_error<E: EngineWrapper>(engine_wrapper: E) {
    let engine = E::standard();

    for num_prefix_quads in 0..50 {
        for num_valid_bytes_penultimate_quad in 0..4 {
            // can't have 1 or it would be invalid length
            for num_pad_bytes_in_final_quad in 2..4 {
                let mut s: String = iter::repeat("ABCD").take(num_prefix_quads).collect();

                // varying amounts of padding in the penultimate quad
                for _ in 0..num_valid_bytes_penultimate_quad {
                    s.push_str("A");
                }
                // finish penultimate quad with padding
                for _ in num_valid_bytes_penultimate_quad..4 {
                    s.push_str("=");
                }
                // and more padding in the final quad
                for _ in 0..num_pad_bytes_in_final_quad {
                    s.push_str("=");
                }

                // padding should be an invalid byte before the final quad.
                // Could argue that the *next* padding byte (in the next quad) is technically the first
                // erroneous one, but reporting that accurately is more complex and probably nobody cares
                assert_eq!(
                    DecodeError::InvalidByte(
                        num_prefix_quads * 4 + num_valid_bytes_penultimate_quad,
                        b'=',
                    ),
                    engine.decode_ez_str_vec(&s).unwrap_err()
                );
            }
        }
    }
}

#[apply(all_engines)]
fn decode_bytes_after_padding_in_final_quad_error<E: EngineWrapper>(engine_wrapper: E) {
    let engine = E::standard();

    for num_prefix_quads in 0..50 {
        for bytes_after_padding in 1..4 {
            let mut s: String = iter::repeat("ABCD").take(num_prefix_quads).collect();

            // every invalid padding position with a 3-byte final quad: 1 to 3 bytes after padding
            for _ in 0..(3 - bytes_after_padding) {
                s.push_str("A");
            }
            s.push_str("=");
            for _ in 0..bytes_after_padding {
                s.push_str("A");
            }

            // First (and only) padding byte is invalid.
            assert_eq!(
                DecodeError::InvalidByte(num_prefix_quads * 4 + (3 - bytes_after_padding), b'='),
                engine.decode_ez_str_vec(&s).unwrap_err()
            );
        }
    }
}

#[apply(all_engines)]
fn decode_absurd_pad_error<E: EngineWrapper>(engine_wrapper: E) {
    let engine = E::standard();

    for num_prefix_quads in 0..50 {
        let mut s: String = iter::repeat("ABCD").take(num_prefix_quads).collect();
        s.push_str("==Y=Wx===pY=2U=====");

        // first padding byte
        assert_eq!(
            DecodeError::InvalidByte(num_prefix_quads * 4, b'='),
            engine.decode_ez_str_vec(&s).unwrap_err()
        );
    }
}

#[apply(all_engines)]
fn decode_too_much_padding_returns_error<E: EngineWrapper>(engine_wrapper: E) {
    let engine = E::standard();

    for num_prefix_quads in 0..50 {
        // add enough padding to ensure that we'll hit all decode stages at the different lengths
        for pad_bytes in 1..64 {
            let mut s: String = iter::repeat("ABCD").take(num_prefix_quads).collect();
            let padding: String = iter::repeat("=").take(pad_bytes).collect();
            s.push_str(&padding);

            if pad_bytes % 4 == 1 {
                assert_eq!(
                    DecodeError::InvalidLength,
                    engine.decode_ez_str_vec(&s).unwrap_err()
                );
            } else {
                assert_eq!(
                    DecodeError::InvalidByte(num_prefix_quads * 4, b'='),
                    engine.decode_ez_str_vec(&s).unwrap_err()
                );
            }
        }
    }
}

#[apply(all_engines)]
fn decode_padding_followed_by_non_padding_returns_error<E: EngineWrapper>(engine_wrapper: E) {
    let engine = E::standard();

    for num_prefix_quads in 0..50 {
        for pad_bytes in 0..32 {
            let mut s: String = iter::repeat("ABCD").take(num_prefix_quads).collect();
            let padding: String = iter::repeat("=").take(pad_bytes).collect();
            s.push_str(&padding);
            s.push_str("E");

            if pad_bytes % 4 == 0 {
                assert_eq!(
                    DecodeError::InvalidLength,
                    engine.decode_ez_str_vec(&s).unwrap_err()
                );
            } else {
                assert_eq!(
                    DecodeError::InvalidByte(num_prefix_quads * 4, b'='),
                    engine.decode_ez_str_vec(&s).unwrap_err()
                );
            }
        }
    }
}

#[apply(all_engines)]
fn decode_one_char_in_final_quad_with_padding_error<E: EngineWrapper>(engine_wrapper: E) {
    let engine = E::standard();

    for num_prefix_quads in 0..50 {
        let mut s: String = iter::repeat("ABCD").take(num_prefix_quads).collect();
        s.push_str("E=");

        assert_eq!(
            DecodeError::InvalidByte(num_prefix_quads * 4 + 1, b'='),
            engine.decode_ez_str_vec(&s).unwrap_err()
        );

        // more padding doesn't change the error
        s.push_str("=");
        assert_eq!(
            DecodeError::InvalidByte(num_prefix_quads * 4 + 1, b'='),
            engine.decode_ez_str_vec(&s).unwrap_err()
        );

        s.push_str("=");
        assert_eq!(
            DecodeError::InvalidByte(num_prefix_quads * 4 + 1, b'='),
            engine.decode_ez_str_vec(&s).unwrap_err()
        );
    }
}

#[apply(all_engines)]
fn decode_too_few_symbols_in_final_quad_error<E: EngineWrapper>(engine_wrapper: E) {
    let engine = E::standard();

    for num_prefix_quads in 0..50 {
        for final_quad_symbols in 0..2 {
            for padding_symbols in 0..(4 - final_quad_symbols) {
                let mut s: String = iter::repeat("ABCD").take(num_prefix_quads).collect();

                for _ in 0..final_quad_symbols {
                    s.push_str("A");
                }
                for _ in 0..padding_symbols {
                    s.push_str("=");
                }

                match final_quad_symbols + padding_symbols {
                    0 => continue,
                    1 => {
                        assert_eq!(
                            DecodeError::InvalidLength,
                            engine.decode_ez_str_vec(&s).unwrap_err()
                        );
                    }
                    _ => {
                        // error reported at first padding byte
                        assert_eq!(
                            DecodeError::InvalidByte(
                                num_prefix_quads * 4 + final_quad_symbols,
                                b'=',
                            ),
                            engine.decode_ez_str_vec(&s).unwrap_err()
                        );
                    }
                }
            }
        }
    }
}

#[apply(all_engines)]
fn decode_invalid_trailing_bytes<E: EngineWrapper>(engine_wrapper: E) {
    let engine = E::standard();

    for num_prefix_quads in 0..50 {
        let mut s: String = iter::repeat("ABCD").take(num_prefix_quads).collect();
        s.push_str("Cg==\n");

        // The case of trailing newlines is common enough to warrant a test for a good error
        // message.
        assert_eq!(
            Err(DecodeError::InvalidByte(num_prefix_quads * 4 + 4, b'\n')),
            engine.decode_ez_str_vec(&s)
        );

        // extra padding, however, is still InvalidLength
        let s = s.replace("\n", "=");
        assert_eq!(
            Err(DecodeError::InvalidLength),
            engine.decode_ez_str_vec(&s)
        );
    }
}

/// Returns a tuple of the original data length, the encoded data length (just data), and the length including padding.
///
/// Vecs provided should be empty.
fn generate_random_encoded_data<E: Engine, R: Rng, D: Distribution<usize>>(
    engine: &E,
    orig_data: &mut Vec<u8>,
    encode_buf: &mut Vec<u8>,
    rng: &mut R,
    length_distribution: &D,
) -> (usize, usize, usize) {
    let padding: bool = rng.gen();

    let orig_len = fill_rand(orig_data, rng, length_distribution);
    let expected_encoded_len = encode::encoded_len(orig_len, padding).unwrap();
    encode_buf.resize(expected_encoded_len, 0);

    let base_encoded_len = engine.encode(&orig_data[..], &mut encode_buf[..]);

    let enc_len_with_padding = if padding {
        base_encoded_len + encode::add_padding(orig_len, &mut encode_buf[base_encoded_len..])
    } else {
        base_encoded_len
    };

    assert_eq!(expected_encoded_len, enc_len_with_padding);

    (orig_len, base_encoded_len, enc_len_with_padding)
}

fn engine_encoded_len(len: usize) -> usize {
    // engines don't pad
    encode::encoded_len(len, false).unwrap()
}

// fill to a random length
fn fill_rand<R: Rng, D: Distribution<usize>>(
    vec: &mut Vec<u8>,
    rng: &mut R,
    length_distribution: &D,
) -> usize {
    let len = length_distribution.sample(rng);
    for _ in 0..len {
        vec.push(rng.gen());
    }

    len
}

fn fill_rand_len<R: Rng>(vec: &mut Vec<u8>, rng: &mut R, len: usize) {
    for _ in 0..len {
        vec.push(rng.gen());
    }
}

fn prefixed_data<'i, 'd>(
    input_with_prefix: &'i mut String,
    prefix_len: usize,
    data: &'d str,
) -> &'i str {
    input_with_prefix.truncate(prefix_len);
    input_with_prefix.push_str(data);
    input_with_prefix.as_str()
}

/// A wrapper to make using engines in rstest fixtures easier.
/// The functions don't need to be instance methods, but rstest does seem
/// to want an instance, so instances are passed to test functions and then ignored.
trait EngineWrapper {
    type Engine: Engine;

    /// Return an engine configured for RFC standard base64
    fn standard() -> Self::Engine;

    /// Return an engine configured for RFC standard base64 that allows invalid trailing bits
    fn standard_forgiving() -> Self::Engine;

    /// Return an engine configured with a randomized alphabet and config
    fn random<R: Rng>(rng: &mut R) -> Self::Engine;

    /// Return an engine configured with the specified alphabet and randomized config
    fn random_alphabet<R: Rng>(rng: &mut R, alphabet: &Alphabet) -> Self::Engine;
}

struct FastPortableWrapper {}

impl EngineWrapper for FastPortableWrapper {
    type Engine = fast_portable::FastPortable;

    fn standard() -> Self::Engine {
        fast_portable::FastPortable::from(&STANDARD, fast_portable::PAD)
    }

    fn standard_forgiving() -> Self::Engine {
        fast_portable::FastPortable::from(
            &STANDARD,
            fast_portable::FastPortableConfig::from(true, true),
        )
    }

    fn random<R: Rng>(rng: &mut R) -> Self::Engine {
        let alphabet = random_alphabet(rng);

        Self::random_alphabet(rng, &alphabet)
    }

    fn random_alphabet<R: Rng>(rng: &mut R, alphabet: &Alphabet) -> Self::Engine {
        let config = fast_portable::FastPortableConfig::from(rng.gen(), rng.gen());

        fast_portable::FastPortable::from(alphabet, config)
    }
}

struct NaiveWrapper {}

impl EngineWrapper for NaiveWrapper {
    type Engine = naive::Naive;

    fn standard() -> Self::Engine {
        naive::Naive::from(
            &STANDARD,
            naive::NaiveConfig {
                padding: true,
                decode_allow_trailing_bits: false,
            },
        )
    }

    fn standard_forgiving() -> Self::Engine {
        naive::Naive::from(
            &STANDARD,
            naive::NaiveConfig {
                padding: true,
                decode_allow_trailing_bits: true,
            },
        )
    }

    fn random<R: Rng>(rng: &mut R) -> Self::Engine {
        let alphabet = random_alphabet(rng);

        Self::random_alphabet(rng, alphabet)
    }

    fn random_alphabet<R: Rng>(rng: &mut R, alphabet: &Alphabet) -> Self::Engine {
        let config = naive::NaiveConfig {
            padding: rng.gen(),
            decode_allow_trailing_bits: rng.gen(),
        };

        naive::Naive::from(alphabet, config)
    }
}

trait EngineExtensions: Engine {
    // a convenience wrapper to avoid the separate estimate call in tests
    fn decode_ez(&self, input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
        let estimate = self.decoded_length_estimate(input.len());

        self.decode(input, output, estimate)
    }

    fn decode_ez_vec(&self, input: &[u8]) -> Result<Vec<u8>, DecodeError> {
        let mut output = Vec::new();
        output.resize((input.len() + 3) / 4 * 3, 0_u8);

        self.decode_ez(input, &mut output[..]).map(|len| {
            // shrink as needed
            output.resize(len, 0_u8);
            output
        })
    }
    fn decode_ez_str_vec(&self, input: &str) -> Result<Vec<u8>, DecodeError> {
        let mut output = Vec::new();
        output.resize((input.len() + 3) / 4 * 3, 0_u8);

        self.decode_ez(input.as_bytes(), &mut output[..])
            .map(|len| {
                // shrink as needed
                output.resize(len, 0_u8);
                output
            })
    }
}

impl<E: Engine> EngineExtensions for E {}
