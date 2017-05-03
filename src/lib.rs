extern crate byteorder;

use std::{fmt, error, ptr, str};

use byteorder::{BigEndian, ByteOrder};

mod tables;

mod line_wrap;
use line_wrap::{line_wrap_parameters, line_wrap};

/// Available encoding character sets
#[derive(Clone, Copy, Debug)]
pub enum CharacterSet {
    /// The standard character set (uses `+` and `/`)
    Standard,
    /// The URL safe character set (uses `-` and `_`)
    UrlSafe
}

impl CharacterSet {
    fn encode_table(&self) -> &'static [u8; 64] {
        match self {
            &CharacterSet::Standard => tables::STANDARD_ENCODE,
            &CharacterSet::UrlSafe => tables::URL_SAFE_ENCODE
        }
    }

    fn decode_table(&self) -> &'static [u8; 256] {
        match self {
            &CharacterSet::Standard => tables::STANDARD_DECODE,
            &CharacterSet::UrlSafe => tables::URL_SAFE_DECODE
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum LineEnding {
    LF,
    CRLF,
}

impl LineEnding {
    fn len(&self) -> usize {
        match self {
            &LineEnding::LF => 1,
            &LineEnding::CRLF => 2
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum LineWrap {
    NoWrap,
    // wrap length is always > 0
    Wrap(usize, LineEnding)
}

/// Contains configuration parameters for base64 encoding
#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Character set to use
    char_set: CharacterSet,
    /// True to pad output with `=` characters
    pad: bool,
    /// Remove whitespace before decoding, at the cost of an allocation
    strip_whitespace: bool,
    /// ADT signifying whether to linewrap output, and if so by how many characters and with what ending
    line_wrap: LineWrap,
}

impl Config {
    pub fn new(char_set: CharacterSet,
               pad: bool,
               strip_whitespace: bool,
               input_line_wrap: LineWrap) -> Config {
        let line_wrap = match input_line_wrap  {
            LineWrap::Wrap(0, _) => LineWrap::NoWrap,
            _ => input_line_wrap,
        };

        Config {
            char_set: char_set,
            pad: pad,
            strip_whitespace: strip_whitespace,
            line_wrap: line_wrap,
        }
    }
}

pub static STANDARD: Config = Config {
    char_set: CharacterSet::Standard,
    pad: true,
    strip_whitespace: false,
    line_wrap: LineWrap::NoWrap,
};

pub static MIME: Config = Config {
    char_set: CharacterSet::Standard,
    pad: true,
    strip_whitespace: true,
    line_wrap: LineWrap::Wrap(76, LineEnding::CRLF),
};

pub static URL_SAFE: Config = Config {
    char_set: CharacterSet::UrlSafe,
    pad: true,
    strip_whitespace: false,
    line_wrap: LineWrap::NoWrap,
};

pub static URL_SAFE_NO_PAD: Config = Config {
    char_set: CharacterSet::UrlSafe,
    pad: false,
    strip_whitespace: false,
    line_wrap: LineWrap::NoWrap,
};


#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    InvalidByte(usize, u8),
    InvalidLength,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DecodeError::InvalidByte(index, byte) =>
                write!(f, "Invalid byte {}, offset {}.", byte, index),
            DecodeError::InvalidLength =>
                write!(f, "Encoded text cannot have a 6-bit remainder.")
        }
    }
}

impl error::Error for DecodeError {
    fn description(&self) -> &str {
        match *self {
            DecodeError::InvalidByte(_, _) => "invalid byte",
            DecodeError::InvalidLength => "invalid length"
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

///Encode arbitrary octets as base64.
///Returns a String.
///Convenience for `encode_config(input, base64::STANDARD);`.
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
pub fn encode<T: ?Sized + AsRef<[u8]>>(input: &T) -> String {
    encode_config(input, STANDARD)
}

///Decode from string reference as octets.
///Returns a Result containing a Vec<u8>.
///Convenience `decode_config(input, base64::STANDARD);`.
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
pub fn decode<T: ?Sized + AsRef<[u8]>>(input: &T) -> Result<Vec<u8>, DecodeError> {
    decode_config(input, STANDARD)
}

///Encode arbitrary octets as base64.
///Returns a String.
///
///# Example
///
///```rust
///extern crate base64;
///
///fn main() {
///    let b64 = base64::encode_config(b"hello world~", base64::STANDARD);
///    println!("{}", b64);
///
///    let b64_url = base64::encode_config(b"hello internet~", base64::URL_SAFE);
///    println!("{}", b64_url);
///}
///```
pub fn encode_config<T: ?Sized + AsRef<[u8]>>(input: &T, config: Config) -> String {
    let mut buf = match encoded_size(input.as_ref().len(), &config) {
        Some(n) => String::with_capacity(n),
        None => panic!("integer overflow when calculating buffer size")
    };

    encode_config_buf(input, config, &mut buf);

    buf
}

/// calculate the base64 encoded string size, including padding
fn encoded_size(bytes_len: usize, config: &Config) -> Option<usize> {
    let rem = bytes_len % 3;

    let complete_input_chunks = bytes_len / 3;
    let complete_chunk_output = complete_input_chunks.checked_mul(4);

    let encoded_len_no_wrap = if rem > 0 {
        if config.pad {
            complete_chunk_output.and_then(|c| c.checked_add(4))
        } else {
            let encoded_rem = match rem {
                1 => 2,
                2 => 3,
                _ => panic!("Impossible remainder")
            };
            complete_chunk_output.and_then(|c| c.checked_add(encoded_rem))
        }
    } else {
        complete_chunk_output
    };

    encoded_len_no_wrap.map(|e| {
        match config.line_wrap {
            LineWrap::NoWrap => e,
            LineWrap::Wrap(line_len, line_ending) => {
                line_wrap_parameters(e, line_len, line_ending).total_len
            }
        }
    })
}

///Encode arbitrary octets as base64.
///Writes into the supplied buffer to avoid allocations.
///
///# Example
///
///```rust
///extern crate base64;
///
///fn main() {
///    let mut buf = String::new();
///    base64::encode_config_buf(b"hello world~", base64::STANDARD, &mut buf);
///    println!("{}", buf);
///
///    buf.clear();
///    base64::encode_config_buf(b"hello internet~", base64::URL_SAFE, &mut buf);
///    println!("{}", buf);
///}
///```
pub fn encode_config_buf<T: ?Sized + AsRef<[u8]>>(input: &T, config: Config, buf: &mut String) {
    let input_bytes = input.as_ref();

    // reserve to make sure the memory we'll be writing to with unsafe is allocated
    let encoded_size = encoded_size(input_bytes.len(), &config)
        .expect("usize overflow when calculating buffer size");

    buf.reserve(encoded_size);

    let orig_buf_len = buf.len();

    // we're only going to insert valid utf8
    let mut buf_bytes;
    unsafe {
        buf_bytes = buf.as_mut_vec();
        // expand the string's vec to use its reserved size
        buf_bytes.set_len(orig_buf_len.checked_add(encoded_size)
            .expect("usize overflow when calculating expanded buffer size"));
    }

    let output_bytes_written = {
        let mut b64_output = &mut buf_bytes[orig_buf_len..];

        // write into the newly reserved space
        let b64_bytes_written = encode_to_slice(input_bytes, b64_output,
                                                    config.char_set.encode_table());

        let padding_bytes = if config.pad {
            add_padding(input_bytes.len(), &mut b64_output[b64_bytes_written..])
        } else {
            0
        };

        let wrappable_bytes = b64_bytes_written.checked_add(padding_bytes)
            .expect("usize overflow when calculating b64 length");

        let line_ending_bytes = match config.line_wrap {
            LineWrap::Wrap(line_len, line_end) =>
                line_wrap(b64_output, wrappable_bytes, line_len, line_end),
            LineWrap::NoWrap => 0
        };

        wrappable_bytes.checked_add(line_ending_bytes)
            .expect("usize overflow when calculating total output length")
    };

    unsafe {
        buf_bytes.set_len(orig_buf_len.checked_add(output_bytes_written)
            .expect("usize overflow when calculating final buffer size"));
    }
}

/// Encode input bytes to utf8 base64 bytes. Does not pad or line wrap.
/// `output` must be long enough to hold the encoded `input` without padding or line wrapping.
/// Returns the number of bytes written.
#[inline]
fn encode_to_slice(input: &[u8], output: &mut [u8], encode_table: &[u8; 64]) -> usize {
    let mut input_index: usize = 0;
    let mut output_ptr = output.as_mut_ptr();

    let last_fast_index = input.len().saturating_sub(8);
    let fast_chunk_len = 6;

    if last_fast_index > 0 {
        while input_index <= last_fast_index {
            let input_chunk = BigEndian::read_u64(&input[input_index..(input_index + 8)]);

            // strip off 6 bits at a time for the first 6 bytes
            unsafe {
                ptr::write(output_ptr, encode_table[((input_chunk >> 58) & 0x3F) as usize]);
                ptr::write(output_ptr.offset(1), encode_table[((input_chunk >> 52) & 0x3F) as usize]);
                ptr::write(output_ptr.offset(2), encode_table[((input_chunk >> 46) & 0x3F) as usize]);
                ptr::write(output_ptr.offset(3), encode_table[((input_chunk >> 40) & 0x3F) as usize]);
                ptr::write(output_ptr.offset(4), encode_table[((input_chunk >> 34) & 0x3F) as usize]);
                ptr::write(output_ptr.offset(5), encode_table[((input_chunk >> 28) & 0x3F) as usize]);
                ptr::write(output_ptr.offset(6), encode_table[((input_chunk >> 22) & 0x3F) as usize]);
                ptr::write(output_ptr.offset(7), encode_table[((input_chunk >> 16) & 0x3F) as usize]);
                output_ptr = output_ptr.offset(8);
            }

            input_index += fast_chunk_len;
        }
    }

    // Encode the 0 to 7 bytes left after the fast loop.

    let rem = input.len() % 3;
    let start_of_rem = input.len() - rem;
    let slow_chunk_len = 3;

    // start at the first index not handled by fast loop, which may be 0.

    while input_index < start_of_rem {
        unsafe {
            ptr::write(output_ptr,
                       encode_table[(input[input_index] >> 2) as usize]);
            ptr::write(output_ptr.offset(1),
                       encode_table[((input[input_index] << 4 | input[input_index + 1] >> 4) & 0x3f) as usize]);
            ptr::write(output_ptr.offset(2),
                       encode_table[((input[input_index + 1] << 2 | input[input_index + 2] >> 6) & 0x3f) as usize]);
            ptr::write(output_ptr.offset(3),
                       encode_table[(input[input_index + 2] & 0x3f) as usize]);
            output_ptr = output_ptr.offset(4);
        }
        input_index += slow_chunk_len;
    }

    if rem == 2 {
        unsafe {
            ptr::write(output_ptr,
                       encode_table[(input[start_of_rem] >> 2) as usize]);
            ptr::write(output_ptr.offset(1),
                       encode_table[((input[start_of_rem] << 4 | input[start_of_rem + 1] >> 4) & 0x3f) as usize]);
            ptr::write(output_ptr.offset(2),
                       encode_table[(input[start_of_rem + 1] << 2 & 0x3f) as usize]);
            output_ptr = output_ptr.offset(3);
        }
    } else if rem == 1 {
        unsafe {
            ptr::write(output_ptr,
                       encode_table[(input[start_of_rem] >> 2) as usize]);
            ptr::write(output_ptr.offset(1),
                       encode_table[(input[start_of_rem] << 4 & 0x3f) as usize]);
            output_ptr = output_ptr.offset(2);
        }
    }

    // will not underflow: output_ptr is always at least as big as the starting output ptr
    (output_ptr as usize) - (output.as_ptr() as usize)
}

/// Write padding characters.
/// `output` is the slice where padding should be written, of length at least 2.
fn add_padding(input_len: usize, output: &mut[u8]) -> usize {
    let rem = input_len % 3;
    let mut bytes_written = 0;
    for _ in 0..((3 - rem) % 3) {
        output[bytes_written] = 0x3d;
        bytes_written += 1;
    }

    bytes_written
}

///Decode from string reference as octets.
///Returns a Result containing a Vec<u8>.
///
///# Example
///
///```rust
///extern crate base64;
///
///fn main() {
///    let bytes = base64::decode_config("aGVsbG8gd29ybGR+Cg==", base64::STANDARD).unwrap();
///    println!("{:?}", bytes);
///
///    let bytes_url = base64::decode_config("aGVsbG8gaW50ZXJuZXR-Cg==", base64::URL_SAFE).unwrap();
///    println!("{:?}", bytes_url);
///}
///```
pub fn decode_config<T: ?Sized + AsRef<[u8]>>(input: &T, config: Config) -> Result<Vec<u8>, DecodeError> {
    let mut buffer = Vec::<u8>::with_capacity(input.as_ref().len() * 4 / 3);

    decode_config_buf(input, config, &mut buffer).map(|_| buffer)
}

///Decode from string reference as octets.
///Writes into the supplied buffer to avoid allocation.
///Returns a Result containing an empty tuple, aka ().
///
///# Example
///
///```rust
///extern crate base64;
///
///fn main() {
///    let mut buffer = Vec::<u8>::new();
///    base64::decode_config_buf("aGVsbG8gd29ybGR+Cg==", base64::STANDARD, &mut buffer).unwrap();
///    println!("{:?}", buffer);
///
///    buffer.clear();
///
///    base64::decode_config_buf("aGVsbG8gaW50ZXJuZXR-Cg==", base64::URL_SAFE, &mut buffer).unwrap();
///    println!("{:?}", buffer);
///}
///```
pub fn decode_config_buf<T: ?Sized + AsRef<[u8]>>(input: &T,
                                                  config: Config,
                                                  buffer: &mut Vec<u8>)
                                                  -> Result<(), DecodeError> {
    let mut input_copy;
    let input_bytes = if config.strip_whitespace {
        input_copy = Vec::<u8>::with_capacity(input.as_ref().len());
        input_copy.extend(input.as_ref().iter().filter(|b| !b" \n\t\r\x0b\x0c".contains(b)));

        input_copy.as_ref()
    } else {
        input.as_ref()
    };

    let ref decode_table = config.char_set.decode_table();

    buffer.reserve(input_bytes.len() * 3 / 4);

    // the fast loop only handles complete chunks of 8 input bytes without padding
    let chunk_len = 8;
    let decoded_chunk_len = 6;
    let remainder_len = input_bytes.len() % chunk_len;
    let trailing_bytes_to_skip = if remainder_len == 0 {
        // if input is a multiple of the chunk size, ignore the last chunk as it may have padding
        chunk_len
    } else {
        remainder_len
    };

    let length_of_full_chunks = input_bytes.len().saturating_sub(trailing_bytes_to_skip);

    let starting_output_index = buffer.len();
    // Resize to hold decoded output from fast loop. Need the extra two bytes because
    // we write a full 8 bytes for the last 6-byte decoded chunk and then truncate off two
    let new_size = starting_output_index
        + length_of_full_chunks / chunk_len * decoded_chunk_len
        + (chunk_len - decoded_chunk_len);
    buffer.resize(new_size, 0);

    let mut output_index = starting_output_index;

    {
        let buffer_slice = buffer.as_mut_slice();

        let mut input_index = 0;
        // initial value is never used; always set if fast loop breaks
        let mut bad_byte_index: usize = 0;
        // a non-invalid value means it's not an error if fast loop never runs
        let mut morsel: u8 = 0;

        // fast loop of 8 bytes at a time
        while input_index < length_of_full_chunks {
            let mut accum: u64;

            let input_chunk = BigEndian::read_u64(&input_bytes[input_index..(input_index + 8)]);
            morsel = decode_table[(input_chunk >> 56) as usize];
            if morsel == tables::INVALID_VALUE {
                bad_byte_index = input_index;
                break;
            };
            accum = (morsel as u64) << 58;

            morsel = decode_table[(input_chunk >> 48 & 0xFF) as usize];
            if morsel == tables::INVALID_VALUE {
                bad_byte_index = input_index + 1;
                break;
            };
            accum |= (morsel as u64) << 52;

            morsel = decode_table[(input_chunk >> 40 & 0xFF) as usize];
            if morsel == tables::INVALID_VALUE {
                bad_byte_index = input_index + 2;
                break;
            };
            accum |= (morsel as u64) << 46;

            morsel = decode_table[(input_chunk >> 32 & 0xFF) as usize];
            if morsel == tables::INVALID_VALUE {
                bad_byte_index = input_index + 3;
                break;
            };
            accum |= (morsel as u64) << 40;

            morsel = decode_table[(input_chunk >> 24 & 0xFF) as usize];
            if morsel == tables::INVALID_VALUE {
                bad_byte_index = input_index + 4;
                break;
            };
            accum |= (morsel as u64) << 34;

            morsel = decode_table[(input_chunk >> 16 & 0xFF) as usize];
            if morsel == tables::INVALID_VALUE {
                bad_byte_index = input_index + 5;
                break;
            };
            accum |= (morsel as u64) << 28;

            morsel = decode_table[(input_chunk >> 8 & 0xFF) as usize];
            if morsel == tables::INVALID_VALUE {
                bad_byte_index = input_index + 6;
                break;
            };
            accum |= (morsel as u64) << 22;

            morsel = decode_table[(input_chunk & 0xFF) as usize];
            if morsel == tables::INVALID_VALUE {
                bad_byte_index = input_index + 7;
                break;
            };
            accum |= (morsel as u64) << 16;

            BigEndian::write_u64(&mut buffer_slice[(output_index)..(output_index + 8)],
                                 accum);

            output_index += 6;
            input_index += chunk_len;
        };

        if morsel == tables::INVALID_VALUE {
            // we got here from a break
            return Err(DecodeError::InvalidByte(bad_byte_index, input_bytes[bad_byte_index]));
        }
    }

    // Truncate off the last two bytes from writing the last u64.
    // Unconditional because we added on the extra 2 bytes in the resize before the loop,
    // so it will never underflow.
    let new_len = buffer.len() - (chunk_len - decoded_chunk_len);
    buffer.truncate(new_len);

    // handle leftovers (at most 8 bytes, decoded to 6).
    // Use a u64 as a stack-resident 8 bytes buffer.
    let mut leftover_bits: u64 = 0;
    let mut morsels_in_leftover = 0;
    let mut padding_bytes = 0;
    let mut first_padding_index: usize = 0;
    for (i, b) in input_bytes[length_of_full_chunks..].iter().enumerate() {
        // '=' padding
        if *b == 0x3D {
            // There can be bad padding in a few ways:
            // 1 - Padding with non-padding characters after it
            // 2 - Padding after zero or one non-padding characters before it
            //     in the current quad.
            // 3 - More than two characters of padding. If 3 or 4 padding chars
            //     are in the same quad, that implies it will be caught by #2.
            //     If it spreads from one quad to another, it will be caught by
            //     #2 in the second quad.

            if i % 4 < 2 {
                // Check for case #2.
                // TODO InvalidPadding error
                return Err(DecodeError::InvalidByte(length_of_full_chunks + i, *b));
            };

            if padding_bytes == 0 {
                first_padding_index = i;
            };

            padding_bytes += 1;
            continue;
        };

        // Check for case #1.
        // To make '=' handling consistent with the main loop, don't allow
        // non-suffix '=' in trailing chunk either. Report error as first
        // erroneous padding.
        if padding_bytes > 0 {
            return Err(DecodeError::InvalidByte(
                length_of_full_chunks + first_padding_index, 0x3D));
        };

        // can use up to 8 * 6 = 48 bits of the u64, if last chunk has no padding.
        // To minimize shifts, pack the leftovers from left to right.
        let shift = 64 - (morsels_in_leftover + 1) * 6;
        // tables are all 256 elements, cannot overflow from a u8 index
        let morsel = decode_table[*b as usize];
        if morsel == tables::INVALID_VALUE {
            return Err(DecodeError::InvalidByte(length_of_full_chunks + i, *b));
        };

        leftover_bits |= (morsel as u64) << shift;
        morsels_in_leftover += 1;
    };

    let leftover_bits_ready_to_append = match morsels_in_leftover {
        0 => 0,
        1 => return Err(DecodeError::InvalidLength),
        2 => 8,
        3 => 16,
        4 => 24,
        5 => return Err(DecodeError::InvalidLength),
        6 => 32,
        7 => 40,
        8 => 48,
        _ => panic!("Impossible: must only have 0 to 4 input bytes in last quad")
    };

    let mut leftover_bits_appended_to_buf = 0;
    while leftover_bits_appended_to_buf < leftover_bits_ready_to_append {
        // `as` simply truncates the higher bits, which is what we want here
        let selected_bits = (leftover_bits >> (56 - leftover_bits_appended_to_buf)) as u8;
        buffer.push(selected_bits);

        leftover_bits_appended_to_buf += 8;
    };

    Ok(())
}

#[cfg(test)]
mod tests;
