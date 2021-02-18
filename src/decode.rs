#[cfg(any(feature = "alloc", feature = "std", test))]
use crate::engine::DecodeEstimate;
use crate::engine::Engine;
#[cfg(any(feature = "alloc", feature = "std", test))]
use crate::engine::DEFAULT_ENGINE;
#[cfg(any(feature = "alloc", feature = "std", test))]
use alloc::vec::Vec;
use core::fmt;
#[cfg(any(feature = "std", test))]
use std::error;

// TODO how to handle InvalidLastSymbol and InvalidLength behavior across engines?

/// Errors that can occur while decoding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// An invalid byte was found in the input. The offset and offending byte are provided.
    InvalidByte(usize, u8),
    /// The length of the input is invalid.
    /// A typical cause of this is stray trailing whitespace or other separator bytes.
    /// In the case where excess trailing bytes have produced an invalid length *and* the last byte
    /// is also an invalid base64 symbol (as would be the case for whitespace, etc), `InvalidByte`
    /// will be emitted instead of `InvalidLength` to make the issue easier to debug.
    InvalidLength,
    /// The last non-padding input symbol's encoded 6 bits have nonzero bits that will be discarded.
    /// This is indicative of corrupted or truncated Base64.
    /// Unlike InvalidByte, which reports symbols that aren't in the alphabet, this error is for
    /// symbols that are in the alphabet but represent nonsensical encodings.
    InvalidLastSymbol(usize, u8),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DecodeError::InvalidByte(index, byte) => {
                write!(f, "Invalid byte {}, offset {}.", byte, index)
            }
            DecodeError::InvalidLength => write!(f, "Encoded text cannot have a 6-bit remainder."),
            DecodeError::InvalidLastSymbol(index, byte) => {
                write!(f, "Invalid last symbol {}, offset {}.", byte, index)
            }
        }
    }
}

#[cfg(any(feature = "std", test))]
impl error::Error for DecodeError {
    fn description(&self) -> &str {
        match *self {
            DecodeError::InvalidByte(_, _) => "invalid byte",
            DecodeError::InvalidLength => "invalid length",
            DecodeError::InvalidLastSymbol(_, _) => "invalid last symbol",
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        None
    }
}

///Decode base64 using the [default engine](DEFAULT_ENGINE), alphabet, and config.
///Returns a `Result` containing a `Vec<u8>`.
///
///# Example
///
///```rust
///extern crate base64;
///
///fn main() {
///    let bytes = base64::decode("aGVsbG8gd29ybGQ=").unwrap();
///    println!("{:?}", bytes);
///}
///```
#[cfg(any(feature = "alloc", feature = "std", test))]
pub fn decode<T: AsRef<[u8]>>(input: T) -> Result<Vec<u8>, DecodeError> {
    decode_engine(input, &DEFAULT_ENGINE)
}

///Decode from string reference as octets using the specified [Engine].
///Returns a `Result` containing a `Vec<u8>`.
///
///# Example
///
///```rust
///extern crate base64;
///
///fn main() {
///    let bytes = base64::decode_engine(
///        "aGVsbG8gd29ybGR+Cg==",
///        &base64::engine::DEFAULT_ENGINE,
///    ).unwrap();
///    println!("{:?}", bytes);
///
///    // custom engine setup
///    let bytes_url = base64::decode_engine(
///        "aGVsbG8gaW50ZXJuZXR-Cg==",
///        &base64::engine::fast_portable::FastPortable::from(
///            &base64::alphabet::URL_SAFE,
///            base64::engine::fast_portable::PAD),
///
///    ).unwrap();
///    println!("{:?}", bytes_url);
///}
///```
#[cfg(any(feature = "alloc", feature = "std", test))]
pub fn decode_engine<E: Engine, T: AsRef<[u8]>>(
    input: T,
    engine: &E,
) -> Result<Vec<u8>, DecodeError> {
    let mut buffer = Vec::<u8>::with_capacity(input.as_ref().len() * 4 / 3);

    decode_engine_vec(input, &mut buffer, engine).map(|_| buffer)
}

///Decode from string reference as octets.
///Writes into the supplied `Vec`, which may allocate if its internal buffer isn't big enough.
///Returns a `Result` containing an empty tuple, aka `()`.
///
///# Example
///
///```rust
///extern crate base64;
///
///const URL_SAFE_ENGINE: base64::engine::fast_portable::FastPortable =
///    base64::engine::fast_portable::FastPortable::from(
///        &base64::alphabet::URL_SAFE,
///        base64::engine::fast_portable::PAD);
///
///fn main() {
///    let mut buffer = Vec::<u8>::new();
///    // with the default engine
///    base64::decode_engine_vec(
///        "aGVsbG8gd29ybGR+Cg==",
///        &mut buffer,
///        &base64::engine::DEFAULT_ENGINE
///    ).unwrap();
///    println!("{:?}", buffer);
///
///    buffer.clear();
///
///    // with a custom engine
///    base64::decode_engine_vec(
///        "aGVsbG8gaW50ZXJuZXR-Cg==",
///        &mut buffer,
///        &URL_SAFE_ENGINE
///    ).unwrap();
///    println!("{:?}", buffer);
///}
///```
#[cfg(any(feature = "alloc", feature = "std", test))]
pub fn decode_engine_vec<'e, 'o, E: Engine, T: AsRef<[u8]>>(
    input: T,
    buffer: &'o mut Vec<u8>,
    engine: &'e E,
) -> Result<(), DecodeError> {
    let input_bytes = input.as_ref();

    let starting_output_len = buffer.len();

    let estimate = engine.decoded_length_estimate(input_bytes.len());
    let total_len_estimate = estimate
        .decoded_length_estimate()
        .checked_add(starting_output_len)
        .expect("Overflow when calculating output buffer length");
    buffer.resize(total_len_estimate, 0);

    let bytes_written;
    {
        let buffer_slice = &mut buffer.as_mut_slice()[starting_output_len..];
        bytes_written = engine.decode(input_bytes, buffer_slice, estimate)?;
    }

    buffer.truncate(starting_output_len + bytes_written);

    Ok(())
}

/// Decode the input into the provided output slice.
///
/// This will not write any bytes past exactly what is decoded (no stray garbage bytes at the end).
///
/// If you don't know ahead of time what the decoded length should be, size your buffer with a
/// conservative estimate for the decoded length of an input: 3 bytes of output for every 4 bytes of
/// input, rounded up, or in other words `(input_len + 3) / 4 * 3`.
///
/// # Panics
///
/// If the slice is not large enough, this will panic.
pub fn decode_engine_slice<'e, 'o, E: Engine, T: AsRef<[u8]>>(
    input: T,
    output: &'o mut [u8],
    engine: &'e E,
) -> Result<usize, DecodeError> {
    let input_bytes = input.as_ref();

    engine.decode(
        input_bytes,
        output,
        engine.decoded_length_estimate(input_bytes.len()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{encode::encode_engine_string, tests::assert_encode_sanity};

    use crate::engine::Config;
    use crate::tests::random_engine;
    use rand::{
        distributions::{Distribution, Uniform},
        FromEntropy, Rng,
    };

    #[test]
    fn decode_into_nonempty_vec_doesnt_clobber_existing_prefix() {
        let mut orig_data = Vec::new();
        let mut encoded_data = String::new();
        let mut decoded_with_prefix = Vec::new();
        let mut decoded_without_prefix = Vec::new();
        let mut prefix = Vec::new();

        let prefix_len_range = Uniform::new(0, 1000);
        let input_len_range = Uniform::new(0, 1000);

        let mut rng = rand::rngs::SmallRng::from_entropy();

        for _ in 0..10_000 {
            orig_data.clear();
            encoded_data.clear();
            decoded_with_prefix.clear();
            decoded_without_prefix.clear();
            prefix.clear();

            let input_len = input_len_range.sample(&mut rng);

            for _ in 0..input_len {
                orig_data.push(rng.gen());
            }

            let engine = random_engine(&mut rng);
            encode_engine_string(&orig_data, &mut encoded_data, &engine);
            assert_encode_sanity(&encoded_data, engine.config().padding(), input_len);

            let prefix_len = prefix_len_range.sample(&mut rng);

            // fill the buf with a prefix
            for _ in 0..prefix_len {
                prefix.push(rng.gen());
            }

            decoded_with_prefix.resize(prefix_len, 0);
            decoded_with_prefix.copy_from_slice(&prefix);

            // decode into the non-empty buf
            decode_engine_vec(&encoded_data, &mut decoded_with_prefix, &engine).unwrap();
            // also decode into the empty buf
            decode_engine_vec(&encoded_data, &mut decoded_without_prefix, &engine).unwrap();

            assert_eq!(
                prefix_len + decoded_without_prefix.len(),
                decoded_with_prefix.len()
            );
            assert_eq!(orig_data, decoded_without_prefix);

            // append plain decode onto prefix
            prefix.append(&mut decoded_without_prefix);

            assert_eq!(prefix, decoded_with_prefix);
        }
    }

    #[test]
    fn decode_into_slice_doesnt_clobber_existing_prefix_or_suffix() {
        let mut orig_data = Vec::new();
        let mut encoded_data = String::new();
        let mut decode_buf = Vec::new();
        let mut decode_buf_copy: Vec<u8> = Vec::new();

        let input_len_range = Uniform::new(0, 1000);

        let mut rng = rand::rngs::SmallRng::from_entropy();

        for _ in 0..10_000 {
            orig_data.clear();
            encoded_data.clear();
            decode_buf.clear();
            decode_buf_copy.clear();

            let input_len = input_len_range.sample(&mut rng);

            for _ in 0..input_len {
                orig_data.push(rng.gen());
            }

            let engine = random_engine(&mut rng);
            encode_engine_string(&orig_data, &mut encoded_data, &engine);
            assert_encode_sanity(&encoded_data, engine.config().padding(), input_len);

            // fill the buffer with random garbage, long enough to have some room before and after
            for _ in 0..5000 {
                decode_buf.push(rng.gen());
            }

            // keep a copy for later comparison
            decode_buf_copy.extend(decode_buf.iter());

            let offset = 1000;

            // decode into the non-empty buf
            let decode_bytes_written =
                decode_engine_slice(&encoded_data, &mut decode_buf[offset..], &engine).unwrap();

            assert_eq!(orig_data.len(), decode_bytes_written);
            assert_eq!(
                orig_data,
                &decode_buf[offset..(offset + decode_bytes_written)]
            );
            assert_eq!(&decode_buf_copy[0..offset], &decode_buf[0..offset]);
            assert_eq!(
                &decode_buf_copy[offset + decode_bytes_written..],
                &decode_buf[offset + decode_bytes_written..]
            );
        }
    }

    #[test]
    fn decode_into_slice_fits_in_precisely_sized_slice() {
        let mut orig_data = Vec::new();
        let mut encoded_data = String::new();
        let mut decode_buf = Vec::new();

        let input_len_range = Uniform::new(0, 1000);

        let mut rng = rand::rngs::SmallRng::from_entropy();

        for _ in 0..10_000 {
            orig_data.clear();
            encoded_data.clear();
            decode_buf.clear();

            let input_len = input_len_range.sample(&mut rng);

            for _ in 0..input_len {
                orig_data.push(rng.gen());
            }

            let engine = random_engine(&mut rng);
            encode_engine_string(&orig_data, &mut encoded_data, &engine);
            assert_encode_sanity(&encoded_data, engine.config().padding(), input_len);

            decode_buf.resize(input_len, 0);

            // decode into the non-empty buf
            let decode_bytes_written =
                decode_engine_slice(&encoded_data, &mut decode_buf[..], &engine).unwrap();

            assert_eq!(orig_data.len(), decode_bytes_written);
            assert_eq!(orig_data, decode_buf);
        }
    }
}
