#[cfg(any(feature = "alloc", feature = "std", test))]
use crate::chunked_encoder;
#[cfg(any(feature = "alloc", feature = "std", test))]
use crate::engine::DEFAULT_ENGINE;
use crate::engine::{Config, Engine};
use crate::PAD_BYTE;
#[cfg(any(feature = "alloc", feature = "std", test))]
use alloc::{string::String, vec};

///Encode arbitrary octets as base64 using the [default engine](DEFAULT_ENGINE), alphabet, and config.
///Returns a `String`.
///
///# Example
///
///```rust
///extern crate base64;
///
///fn main() {
///    let b64 = base64::encode(b"hello world");
///    println!("{}", b64);
///}
///```
#[cfg(any(feature = "alloc", feature = "std", test))]
pub fn encode<T: AsRef<[u8]>>(input: T) -> String {
    encode_engine(input, &DEFAULT_ENGINE)
}

///Encode arbitrary octets as base64 using the provided `Engine`.
///Returns a `String`.
///
///# Example
///
///```rust
///extern crate base64;
///
///const URL_SAFE_ENGINE: base64::engine::fast_portable::FastPortable =
///    base64::engine::fast_portable::FastPortable::from(
///        &base64::alphabet::URL_SAFE,
///        base64::engine::fast_portable::NO_PAD);
///
///fn main() {
///    let b64 = base64::encode_engine(
///        b"hello world~",
///        &base64::engine::DEFAULT_ENGINE
///        );
///    println!("{}", b64);
///
///    let b64_url = base64::encode_engine(
///        b"hello internet~",
///        &URL_SAFE_ENGINE
///        );
///    println!("{}", b64_url);
///}
///```
#[cfg(any(feature = "alloc", feature = "std", test))]
pub fn encode_engine<E: Engine, T: AsRef<[u8]>>(input: T, engine: &E) -> String {
    let encoded_size = encoded_len(input.as_ref().len(), engine.config().padding())
        .expect("integer overflow when calculating buffer size");
    let mut buf = vec![0; encoded_size];

    encode_with_padding(input.as_ref(), &mut buf[..], engine, encoded_size);

    String::from_utf8(buf).expect("Invalid UTF8")
}

///Encode arbitrary octets as base64.
///Writes into the supplied `String`, which may allocate if its internal buffer isn't big enough.
///
///# Example
///
///```rust
///extern crate base64;
///
///const URL_SAFE_ENGINE: base64::engine::fast_portable::FastPortable =
///    base64::engine::fast_portable::FastPortable::from(
///        &base64::alphabet::URL_SAFE,
///        base64::engine::fast_portable::NO_PAD);
///fn main() {
///    let mut buf = String::new();
///    base64::encode_engine_string(
///        b"hello world~",
///        &mut buf,
///        &base64::engine::DEFAULT_ENGINE);
///    println!("{}", buf);
///
///    buf.clear();
///    base64::encode_engine_string(
///        b"hello internet~",
///        &mut buf,
///        &URL_SAFE_ENGINE);
///    println!("{}", buf);
///}
///```
#[cfg(any(feature = "alloc", feature = "std", test))]
pub fn encode_engine_string<E: Engine, T: AsRef<[u8]>>(
    input: T,
    output_buf: &mut String,
    engine: &E,
) {
    let input_bytes = input.as_ref();

    {
        let mut sink = chunked_encoder::StringSink::from(output_buf);
        let encoder = chunked_encoder::ChunkedEncoder::from(engine);

        encoder
            .encode(input_bytes, &mut sink)
            .expect("Writing to a String shouldn't fail")
    }
}

/// Encode arbitrary octets as base64.
/// Writes into the supplied output buffer.
///
/// This is useful if you wish to avoid allocation entirely (e.g. encoding into a stack-resident
/// or statically-allocated buffer).
///
/// # Panics
///
/// If `output` is too small to hold the encoded version of `input`, a panic will result.
///
/// # Example
///
/// ```rust
/// extern crate base64;
///
/// fn main() {
///     let s = b"hello internet!";
///     let mut buf = Vec::new();
///     // make sure we'll have a slice big enough for base64 + padding
///     buf.resize(s.len() * 4 / 3 + 4, 0);
///
///     let bytes_written = base64::encode_engine_slice(
///         s,
///         &mut buf,
///         &base64::engine::DEFAULT_ENGINE);
///
///     // shorten our vec down to just what was written
///     buf.resize(bytes_written, 0);
///
///     assert_eq!(s, base64::decode(&buf).unwrap().as_slice());
/// }
/// ```
pub fn encode_engine_slice<E: Engine, T: AsRef<[u8]>>(
    input: T,
    output_buf: &mut [u8],
    engine: &E,
) -> usize {
    let input_bytes = input.as_ref();

    let encoded_size = encoded_len(input_bytes.len(), engine.config().padding())
        .expect("usize overflow when calculating buffer size");

    let mut b64_output = &mut output_buf[0..encoded_size];

    encode_with_padding(&input_bytes, &mut b64_output, engine, encoded_size);

    encoded_size
}

/// B64-encode and pad (if configured).
///
/// This helper exists to avoid recalculating encoded_size, which is relatively expensive on short
/// inputs.
///
/// `encoded_size` is the encoded size calculated for `input`.
///
/// `output` must be of size `encoded_size`.
///
/// All bytes in `output` will be written to since it is exactly the size of the output.
fn encode_with_padding<E: Engine>(
    input: &[u8],
    output: &mut [u8],
    engine: &E,
    expected_encoded_size: usize,
) {
    debug_assert_eq!(expected_encoded_size, output.len());

    let b64_bytes_written = engine.encode(input, output);

    let padding_bytes = if engine.config().padding() {
        add_padding(input.len(), &mut output[b64_bytes_written..])
    } else {
        0
    };

    let encoded_bytes = b64_bytes_written
        .checked_add(padding_bytes)
        .expect("usize overflow when calculating b64 length");

    debug_assert_eq!(expected_encoded_size, encoded_bytes);
}

/// calculate the base64 encoded string size, including padding if appropriate
pub fn encoded_len(bytes_len: usize, padding: bool) -> Option<usize> {
    let rem = bytes_len % 3;

    let complete_input_chunks = bytes_len / 3;
    let complete_chunk_output = complete_input_chunks.checked_mul(4);

    if rem > 0 {
        if padding {
            complete_chunk_output.and_then(|c| c.checked_add(4))
        } else {
            let encoded_rem = match rem {
                1 => 2,
                2 => 3,
                _ => unreachable!("Impossible remainder"),
            };
            complete_chunk_output.and_then(|c| c.checked_add(encoded_rem))
        }
    } else {
        complete_chunk_output
    }
}

/// Write padding characters.
/// `input_len` is the size of the original, not encoded, input.
/// `output` is the slice where padding should be written, of length at least 2.
///
/// Returns the number of padding bytes written.
pub fn add_padding(input_len: usize, output: &mut [u8]) -> usize {
    // TODO base on encoded len to use cheaper mod by 4 (aka & 7)
    let rem = input_len % 3;
    let mut bytes_written = 0;
    for _ in 0..((3 - rem) % 3) {
        output[bytes_written] = PAD_BYTE;
        bytes_written += 1;
    }

    bytes_written
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        decode::decode_engine_vec,
        tests::{assert_encode_sanity, random_config},
    };

    use crate::alphabet::{IMAP_MUTF7, STANDARD, URL_SAFE};
    use crate::engine::fast_portable::{FastPortable, NO_PAD};
    use crate::tests::random_engine;
    use rand::{
        distributions::{Distribution, Uniform},
        FromEntropy, Rng,
    };
    use std;
    use std::str;

    const URL_SAFE_NO_PAD_ENGINE: FastPortable = FastPortable::from(&URL_SAFE, NO_PAD);

    #[test]
    fn encoded_size_correct_standard() {
        assert_encoded_length(0, 0, &DEFAULT_ENGINE, true);

        assert_encoded_length(1, 4, &DEFAULT_ENGINE, true);
        assert_encoded_length(2, 4, &DEFAULT_ENGINE, true);
        assert_encoded_length(3, 4, &DEFAULT_ENGINE, true);

        assert_encoded_length(4, 8, &DEFAULT_ENGINE, true);
        assert_encoded_length(5, 8, &DEFAULT_ENGINE, true);
        assert_encoded_length(6, 8, &DEFAULT_ENGINE, true);

        assert_encoded_length(7, 12, &DEFAULT_ENGINE, true);
        assert_encoded_length(8, 12, &DEFAULT_ENGINE, true);
        assert_encoded_length(9, 12, &DEFAULT_ENGINE, true);

        assert_encoded_length(54, 72, &DEFAULT_ENGINE, true);

        assert_encoded_length(55, 76, &DEFAULT_ENGINE, true);
        assert_encoded_length(56, 76, &DEFAULT_ENGINE, true);
        assert_encoded_length(57, 76, &DEFAULT_ENGINE, true);

        assert_encoded_length(58, 80, &DEFAULT_ENGINE, true);
    }

    #[test]
    fn encoded_size_correct_no_pad() {
        assert_encoded_length(0, 0, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(1, 2, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(2, 3, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(3, 4, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(4, 6, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(5, 7, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(6, 8, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(7, 10, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(8, 11, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(9, 12, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(54, 72, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(55, 74, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(56, 75, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(57, 76, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(58, 78, &URL_SAFE_NO_PAD_ENGINE, false);
    }

    #[test]
    fn encoded_size_overflow() {
        assert_eq!(None, encoded_len(std::usize::MAX, true));
    }

    #[test]
    fn encode_engine_string_into_nonempty_buffer_doesnt_clobber_prefix() {
        let mut orig_data = Vec::new();
        let mut prefix = String::new();
        let mut encoded_data_no_prefix = String::new();
        let mut encoded_data_with_prefix = String::new();
        let mut decoded = Vec::new();

        let prefix_len_range = Uniform::new(0, 1000);
        let input_len_range = Uniform::new(0, 1000);

        let mut rng = rand::rngs::SmallRng::from_entropy();

        for _ in 0..10_000 {
            orig_data.clear();
            prefix.clear();
            encoded_data_no_prefix.clear();
            encoded_data_with_prefix.clear();
            decoded.clear();

            let input_len = input_len_range.sample(&mut rng);

            for _ in 0..input_len {
                orig_data.push(rng.gen());
            }

            let prefix_len = prefix_len_range.sample(&mut rng);
            for _ in 0..prefix_len {
                // getting convenient random single-byte printable chars that aren't base64 is
                // annoying
                prefix.push('#');
            }
            encoded_data_with_prefix.push_str(&prefix);

            let engine = random_engine(&mut rng);
            encode_engine_string(&orig_data, &mut encoded_data_no_prefix, &engine);
            encode_engine_string(&orig_data, &mut encoded_data_with_prefix, &engine);

            assert_eq!(
                encoded_data_no_prefix.len() + prefix_len,
                encoded_data_with_prefix.len()
            );
            assert_encode_sanity(
                &encoded_data_no_prefix,
                engine.config().padding(),
                input_len,
            );
            assert_encode_sanity(
                &encoded_data_with_prefix[prefix_len..],
                engine.config().padding(),
                input_len,
            );

            // append plain encode onto prefix
            prefix.push_str(&mut encoded_data_no_prefix);

            assert_eq!(prefix, encoded_data_with_prefix);

            decode_engine_vec(&encoded_data_no_prefix, &mut decoded, &engine).unwrap();
            assert_eq!(orig_data, decoded);
        }
    }

    #[test]
    fn encode_engine_slice_into_nonempty_buffer_doesnt_clobber_suffix() {
        let mut orig_data = Vec::new();
        let mut encoded_data = Vec::new();
        let mut encoded_data_original_state = Vec::new();
        let mut decoded = Vec::new();

        let input_len_range = Uniform::new(0, 1000);

        let mut rng = rand::rngs::SmallRng::from_entropy();

        for _ in 0..10_000 {
            orig_data.clear();
            encoded_data.clear();
            encoded_data_original_state.clear();
            decoded.clear();

            let input_len = input_len_range.sample(&mut rng);

            for _ in 0..input_len {
                orig_data.push(rng.gen());
            }

            // plenty of existing garbage in the encoded buffer
            for _ in 0..10 * input_len {
                encoded_data.push(rng.gen());
            }

            encoded_data_original_state.extend_from_slice(&encoded_data);

            let engine = random_engine(&mut rng);

            let encoded_size = encoded_len(input_len, engine.config().padding()).unwrap();

            assert_eq!(
                encoded_size,
                encode_engine_slice(&orig_data, &mut encoded_data, &engine)
            );

            assert_encode_sanity(
                std::str::from_utf8(&encoded_data[0..encoded_size]).unwrap(),
                engine.config().padding(),
                input_len,
            );

            assert_eq!(
                &encoded_data[encoded_size..],
                &encoded_data_original_state[encoded_size..]
            );

            decode_engine_vec(&encoded_data[0..encoded_size], &mut decoded, &engine).unwrap();
            assert_eq!(orig_data, decoded);
        }
    }

    #[test]
    fn encode_engine_slice_fits_into_precisely_sized_slice() {
        let mut orig_data = Vec::new();
        let mut encoded_data = Vec::new();
        let mut decoded = Vec::new();

        let input_len_range = Uniform::new(0, 1000);

        let mut rng = rand::rngs::SmallRng::from_entropy();

        for _ in 0..10_000 {
            orig_data.clear();
            encoded_data.clear();
            decoded.clear();

            let input_len = input_len_range.sample(&mut rng);

            for _ in 0..input_len {
                orig_data.push(rng.gen());
            }

            let engine = random_engine(&mut rng);

            let encoded_size = encoded_len(input_len, engine.config().padding()).unwrap();

            encoded_data.resize(encoded_size, 0);

            assert_eq!(
                encoded_size,
                encode_engine_slice(&orig_data, &mut encoded_data, &engine)
            );

            assert_encode_sanity(
                std::str::from_utf8(&encoded_data[0..encoded_size]).unwrap(),
                engine.config().padding(),
                input_len,
            );

            decode_engine_vec(&encoded_data[0..encoded_size], &mut decoded, &engine).unwrap();
            assert_eq!(orig_data, decoded);
        }
    }

    #[test]
    fn encode_to_slice_random_valid_utf8() {
        let mut input = Vec::new();
        let mut output = Vec::new();

        let input_len_range = Uniform::new(0, 1000);

        let mut rng = rand::rngs::SmallRng::from_entropy();

        for _ in 0..10_000 {
            input.clear();
            output.clear();

            let input_len = input_len_range.sample(&mut rng);

            for _ in 0..input_len {
                input.push(rng.gen());
            }

            let config = random_config(&mut rng);
            let engine = random_engine(&mut rng);

            // fill up the output buffer with garbage
            let encoded_size = encoded_len(input_len, config.padding()).unwrap();
            for _ in 0..encoded_size {
                output.push(rng.gen());
            }

            let orig_output_buf = output.to_vec();

            let bytes_written = engine.encode(&input, &mut output);

            // make sure the part beyond bytes_written is the same garbage it was before
            assert_eq!(orig_output_buf[bytes_written..], output[bytes_written..]);

            // make sure the encoded bytes are UTF-8
            let _ = str::from_utf8(&output[0..bytes_written]).unwrap();
        }
    }

    #[test]
    fn encode_with_padding_random_valid_utf8() {
        let mut input = Vec::new();
        let mut output = Vec::new();

        let input_len_range = Uniform::new(0, 1000);

        let mut rng = rand::rngs::SmallRng::from_entropy();

        for _ in 0..10_000 {
            input.clear();
            output.clear();

            let input_len = input_len_range.sample(&mut rng);

            for _ in 0..input_len {
                input.push(rng.gen());
            }

            let engine = random_engine(&mut rng);

            // fill up the output buffer with garbage
            let encoded_size = encoded_len(input_len, engine.config().padding()).unwrap();
            for _ in 0..encoded_size + 1000 {
                output.push(rng.gen());
            }

            let orig_output_buf = output.to_vec();

            encode_with_padding(&input, &mut output[0..encoded_size], &engine, encoded_size);

            // make sure the part beyond b64 is the same garbage it was before
            assert_eq!(orig_output_buf[encoded_size..], output[encoded_size..]);

            // make sure the encoded bytes are UTF-8
            let _ = str::from_utf8(&output[0..encoded_size]).unwrap();
        }
    }

    #[test]
    fn add_padding_random_valid_utf8() {
        let mut output = Vec::new();

        let mut rng = rand::rngs::SmallRng::from_entropy();

        // cover our bases for length % 3
        for input_len in 0..10 {
            output.clear();

            // fill output with random
            for _ in 0..10 {
                output.push(rng.gen());
            }

            let orig_output_buf = output.to_vec();

            let bytes_written = add_padding(input_len, &mut output);

            // make sure the part beyond bytes_written is the same garbage it was before
            assert_eq!(orig_output_buf[bytes_written..], output[bytes_written..]);

            // make sure the encoded bytes are UTF-8
            let _ = str::from_utf8(&output[0..bytes_written]).unwrap();
        }
    }

    fn assert_encoded_length<E: Engine>(
        input_len: usize,
        enc_len: usize,
        engine: &E,
        padded: bool,
    ) {
        assert_eq!(enc_len, encoded_len(input_len, padded).unwrap());

        let mut bytes: Vec<u8> = Vec::new();
        let mut rng = rand::rngs::SmallRng::from_entropy();

        for _ in 0..input_len {
            bytes.push(rng.gen());
        }

        let encoded = encode_engine(&bytes, engine);
        assert_encode_sanity(&encoded, padded, input_len);

        assert_eq!(enc_len, encoded.len());
    }

    #[test]
    fn encode_imap() {
        assert_eq!(
            encode_engine(b"\xFB\xFF", &FastPortable::from(&IMAP_MUTF7, NO_PAD)),
            encode_engine(b"\xFB\xFF", &FastPortable::from(&STANDARD, NO_PAD)).replace("/", ",")
        );
    }
}
