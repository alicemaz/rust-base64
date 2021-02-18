//! Provides the [FastPortable] engine and associated config types.
use crate::alphabet::Alphabet;
use crate::engine::Config;
use crate::DecodeError;
use core::convert::TryInto;

mod decode;
pub use decode::FastPortableEstimate;

pub(crate) const INVALID_VALUE: u8 = 255;

/// A general-purpose base64 engine.
///
/// - It uses no vector CPU instructions, so it will work on any system.
/// - It is reasonably fast (~2GiB/s).
/// - It is not constant-time, though, so it is vulnerable to timing side-channel attacks. For loading cryptographic keys, etc, it is suggested to use the forthcoming constant-time implementation.
pub struct FastPortable {
    encode_table: [u8; 64],
    decode_table: [u8; 256],
    config: FastPortableConfig,
}

impl FastPortable {
    /// Create a `FastPortable` engine from an [Alphabet].
    ///
    /// While not very expensive to initialize, ideally these should be cached
    /// if the engine will be used repeatedly.
    pub const fn from(alphabet: &Alphabet, config: FastPortableConfig) -> FastPortable {
        FastPortable {
            encode_table: encode_table(&alphabet),
            decode_table: decode_table(&alphabet),
            config,
        }
    }
}

impl super::Engine for FastPortable {
    type Config = FastPortableConfig;
    type DecodeEstimate = FastPortableEstimate;

    fn encode(&self, input: &[u8], output: &mut [u8]) -> usize {
        let mut input_index: usize = 0;

        const BLOCKS_PER_FAST_LOOP: usize = 4;
        const LOW_SIX_BITS: u64 = 0x3F;

        // we read 8 bytes at a time (u64) but only actually consume 6 of those bytes. Thus, we need
        // 2 trailing bytes to be available to read..
        let last_fast_index = input.len().saturating_sub(BLOCKS_PER_FAST_LOOP * 6 + 2);
        let mut output_index = 0;

        if last_fast_index > 0 {
            while input_index <= last_fast_index {
                // Major performance wins from letting the optimizer do the bounds check once, mostly
                // on the output side
                let input_chunk =
                    &input[input_index..(input_index + (BLOCKS_PER_FAST_LOOP * 6 + 2))];
                let output_chunk =
                    &mut output[output_index..(output_index + BLOCKS_PER_FAST_LOOP * 8)];

                // Hand-unrolling for 32 vs 16 or 8 bytes produces yields performance about equivalent
                // to unsafe pointer code on a Xeon E5-1650v3. 64 byte unrolling was slightly better for
                // large inputs but significantly worse for 50-byte input, unsurprisingly. I suspect
                // that it's a not uncommon use case to encode smallish chunks of data (e.g. a 64-byte
                // SHA-512 digest), so it would be nice if that fit in the unrolled loop at least once.
                // Plus, single-digit percentage performance differences might well be quite different
                // on different hardware.

                let input_u64 = read_u64(&input_chunk[0..]);

                output_chunk[0] = self.encode_table[((input_u64 >> 58) & LOW_SIX_BITS) as usize];
                output_chunk[1] = self.encode_table[((input_u64 >> 52) & LOW_SIX_BITS) as usize];
                output_chunk[2] = self.encode_table[((input_u64 >> 46) & LOW_SIX_BITS) as usize];
                output_chunk[3] = self.encode_table[((input_u64 >> 40) & LOW_SIX_BITS) as usize];
                output_chunk[4] = self.encode_table[((input_u64 >> 34) & LOW_SIX_BITS) as usize];
                output_chunk[5] = self.encode_table[((input_u64 >> 28) & LOW_SIX_BITS) as usize];
                output_chunk[6] = self.encode_table[((input_u64 >> 22) & LOW_SIX_BITS) as usize];
                output_chunk[7] = self.encode_table[((input_u64 >> 16) & LOW_SIX_BITS) as usize];

                let input_u64 = read_u64(&input_chunk[6..]);

                output_chunk[8] = self.encode_table[((input_u64 >> 58) & LOW_SIX_BITS) as usize];
                output_chunk[9] = self.encode_table[((input_u64 >> 52) & LOW_SIX_BITS) as usize];
                output_chunk[10] = self.encode_table[((input_u64 >> 46) & LOW_SIX_BITS) as usize];
                output_chunk[11] = self.encode_table[((input_u64 >> 40) & LOW_SIX_BITS) as usize];
                output_chunk[12] = self.encode_table[((input_u64 >> 34) & LOW_SIX_BITS) as usize];
                output_chunk[13] = self.encode_table[((input_u64 >> 28) & LOW_SIX_BITS) as usize];
                output_chunk[14] = self.encode_table[((input_u64 >> 22) & LOW_SIX_BITS) as usize];
                output_chunk[15] = self.encode_table[((input_u64 >> 16) & LOW_SIX_BITS) as usize];

                let input_u64 = read_u64(&input_chunk[12..]);

                output_chunk[16] = self.encode_table[((input_u64 >> 58) & LOW_SIX_BITS) as usize];
                output_chunk[17] = self.encode_table[((input_u64 >> 52) & LOW_SIX_BITS) as usize];
                output_chunk[18] = self.encode_table[((input_u64 >> 46) & LOW_SIX_BITS) as usize];
                output_chunk[19] = self.encode_table[((input_u64 >> 40) & LOW_SIX_BITS) as usize];
                output_chunk[20] = self.encode_table[((input_u64 >> 34) & LOW_SIX_BITS) as usize];
                output_chunk[21] = self.encode_table[((input_u64 >> 28) & LOW_SIX_BITS) as usize];
                output_chunk[22] = self.encode_table[((input_u64 >> 22) & LOW_SIX_BITS) as usize];
                output_chunk[23] = self.encode_table[((input_u64 >> 16) & LOW_SIX_BITS) as usize];

                let input_u64 = read_u64(&input_chunk[18..]);

                output_chunk[24] = self.encode_table[((input_u64 >> 58) & LOW_SIX_BITS) as usize];
                output_chunk[25] = self.encode_table[((input_u64 >> 52) & LOW_SIX_BITS) as usize];
                output_chunk[26] = self.encode_table[((input_u64 >> 46) & LOW_SIX_BITS) as usize];
                output_chunk[27] = self.encode_table[((input_u64 >> 40) & LOW_SIX_BITS) as usize];
                output_chunk[28] = self.encode_table[((input_u64 >> 34) & LOW_SIX_BITS) as usize];
                output_chunk[29] = self.encode_table[((input_u64 >> 28) & LOW_SIX_BITS) as usize];
                output_chunk[30] = self.encode_table[((input_u64 >> 22) & LOW_SIX_BITS) as usize];
                output_chunk[31] = self.encode_table[((input_u64 >> 16) & LOW_SIX_BITS) as usize];

                output_index += BLOCKS_PER_FAST_LOOP * 8;
                input_index += BLOCKS_PER_FAST_LOOP * 6;
            }
        }

        // Encode what's left after the fast loop.

        const LOW_SIX_BITS_U8: u8 = 0x3F;

        let rem = input.len() % 3;
        let start_of_rem = input.len() - rem;

        // start at the first index not handled by fast loop, which may be 0.

        while input_index < start_of_rem {
            let input_chunk = &input[input_index..(input_index + 3)];
            let output_chunk = &mut output[output_index..(output_index + 4)];

            output_chunk[0] = self.encode_table[(input_chunk[0] >> 2) as usize];
            output_chunk[1] = self.encode_table
                [((input_chunk[0] << 4 | input_chunk[1] >> 4) & LOW_SIX_BITS_U8) as usize];
            output_chunk[2] = self.encode_table
                [((input_chunk[1] << 2 | input_chunk[2] >> 6) & LOW_SIX_BITS_U8) as usize];
            output_chunk[3] = self.encode_table[(input_chunk[2] & LOW_SIX_BITS_U8) as usize];

            input_index += 3;
            output_index += 4;
        }

        if rem == 2 {
            output[output_index] = self.encode_table[(input[start_of_rem] >> 2) as usize];
            output[output_index + 1] =
                self.encode_table[((input[start_of_rem] << 4 | input[start_of_rem + 1] >> 4)
                    & LOW_SIX_BITS_U8) as usize];
            output[output_index + 2] =
                self.encode_table[((input[start_of_rem + 1] << 2) & LOW_SIX_BITS_U8) as usize];
            output_index += 3;
        } else if rem == 1 {
            output[output_index] = self.encode_table[(input[start_of_rem] >> 2) as usize];
            output[output_index + 1] =
                self.encode_table[((input[start_of_rem] << 4) & LOW_SIX_BITS_U8) as usize];
            output_index += 2;
        }

        output_index
    }

    fn decoded_length_estimate(&self, input_len: usize) -> Self::DecodeEstimate {
        FastPortableEstimate::from(input_len)
    }

    fn decode(
        &self,
        input: &[u8],
        output: &mut [u8],
        estimate: Self::DecodeEstimate,
    ) -> Result<usize, DecodeError> {
        decode::decode_helper(
            input,
            estimate,
            output,
            &self.decode_table,
            self.config.decode_allow_trailing_bits,
        )
    }

    fn config(&self) -> Self::Config {
        self.config
    }
}

/// Returns a table mapping a 6-bit index to the ASCII byte encoding of the index
pub(crate) const fn encode_table(alphabet: &Alphabet) -> [u8; 64] {
    // the encode table is just the alphabet:
    // 6-bit index lookup -> printable byte
    let mut encode_table = [0_u8; 64];
    {
        let mut index = 0;
        while index < 64 {
            encode_table[index] = alphabet.symbols[index];
            index += 1;
        }
    }

    return encode_table;
}

/// Returns a table mapping base64 bytes as the lookup index to either:
/// - [INVALID_VALUE] for bytes that aren't members of the alphabet
/// - a byte whose lower 6 bits are the value that was encoded into the index byte
pub(crate) const fn decode_table(alphabet: &Alphabet) -> [u8; 256] {
    let mut decode_table = [INVALID_VALUE; 256];

    // Since the table is full of `INVALID_VALUE` already, we only need to overwrite
    // the parts that are valid.
    let mut index = 0;
    while index < 64 {
        // The index in the alphabet is the 6-bit value we care about.
        // Since the index is in 0-63, it is safe to cast to u8.
        decode_table[alphabet.symbols[index] as usize] = index as u8;
        index += 1;
    }

    return decode_table;
}

// fn decode_aligned(symbol: u8, decode_table: &[u8; 256]) -> u8 {
//     let mut result: u8 = 0x00;
//     // If `symbol` is inside the printable range, one of these two derived indices will be equal to
//     // the original index, and the decoded byte will end up in `result`. If `symbol` is not
//     // printable, neither will equal the original symbol, and so both decoded bytes will have 0x00
//     // as a mask.
//     // TODO invalid bytes decoded to 0x00 instead of 0xFF?
//     let idx: [u8; 2] = [symbol % 64, symbol % 64 + 64];
//     for i in 0..2 {
//         let symbol_eq_mod = idx[i] == symbol;
//         // if symbol equals its mod flavor, 0xFF, else 0x00
//         let mask = ((symbol_eq_mod) as i8 - 1) as u8;
//         result = result | (decode_table[idx[i] as usize] & mask);
//     }
//     result
// }

#[inline]
fn read_u64(s: &[u8]) -> u64 {
    u64::from_be_bytes(s[..8].try_into().unwrap())
}

/// Contains miscellaneous configuration parameters for base64 encoding and decoding.
///
/// To specify the characters used, see [crate::alphabet::Alphabet].
#[derive(Clone, Copy, Debug)]
pub struct FastPortableConfig {
    /// `true` to pad output with `=` characters
    padding: bool,
    /// `true` to ignore excess nonzero bits in the last few symbols, otherwise an error is returned
    decode_allow_trailing_bits: bool,
}

impl FastPortableConfig {
    /// Create a new config.
    ///
    /// - `padding`: if `true`, encoding will append `=` padding characters to produce an
    /// output whose length is a multiple of 4. Padding is not needed for decoding and
    /// only serves to waste bytes but it's in the spec. For new applications, consider
    /// not using padding.
    /// - `decode_allow_trailing_bits`: If unsure, use `false`.
    /// Useful if you need to decode base64 produced by a buggy encoder that
    /// has bits set in the unused space on the last base64 character as per
    /// [forgiving-base64 decode](https://infra.spec.whatwg.org/#forgiving-base64-decode).
    /// If invalid trailing bits are present and this `true`, those bits will
    /// be silently ignored, else `DecodeError::InvalidLastSymbol` will be emitted.
    pub const fn from(padding: bool, decode_allow_trailing_bits: bool) -> FastPortableConfig {
        FastPortableConfig {
            padding,
            decode_allow_trailing_bits,
        }
    }

    /// Create a new `Config` based on `self` with an updated `padding` parameter.
    pub const fn with_padding(self, padding: bool) -> FastPortableConfig {
        FastPortableConfig { padding, ..self }
    }

    /// Create a new `Config` based on `self` with an updated `decode_allow_trailing_bits` parameter.
    pub const fn with_decode_allow_trailing_bits(self, allow: bool) -> FastPortableConfig {
        FastPortableConfig {
            decode_allow_trailing_bits: allow,
            ..self
        }
    }
}

impl Config for FastPortableConfig {
    fn padding(&self) -> bool {
        self.padding
    }
}

/// Include padding bytes when encoding.
pub const PAD: FastPortableConfig = FastPortableConfig {
    padding: true,
    decode_allow_trailing_bits: false,
};

/// Don't add padding when encoding.
pub const NO_PAD: FastPortableConfig = FastPortableConfig {
    padding: false,
    decode_allow_trailing_bits: false,
};
