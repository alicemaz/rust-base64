use crate::alphabet::Alphabet;
use crate::engine::fast_portable::{decode_table, encode_table};
use crate::engine::{fast_portable, Config, DecodeEstimate, Engine};
use crate::{DecodeError, PAD_BYTE};
use alloc::ops::BitOr;
use std::ops::{BitAnd, Shl, Shr};

/// Comparatively simple implementation that can be used as something to compare against in tests
pub struct Naive {
    encode_table: [u8; 64],
    decode_table: [u8; 256],
    config: NaiveConfig,
}

impl Naive {
    const ENCODE_INPUT_CHUNK_SIZE: usize = 3;
    const DECODE_INPUT_CHUNK_SIZE: usize = 4;

    pub const fn from(alphabet: &Alphabet, config: NaiveConfig) -> Naive {
        Naive {
            encode_table: encode_table(&alphabet),
            decode_table: decode_table(&alphabet),
            config,
        }
    }

    fn decode_byte_into_u32(&self, offset: usize, byte: u8) -> Result<u32, DecodeError> {
        let decoded = self.decode_table[byte as usize];

        if decoded == fast_portable::INVALID_VALUE {
            return Err(DecodeError::InvalidByte(offset, byte));
        }

        Ok(decoded as u32)
    }
}

impl Engine for Naive {
    type Config = NaiveConfig;
    type DecodeEstimate = NaiveEstimate;

    fn encode(&self, input: &[u8], output: &mut [u8]) -> usize {
        // complete chunks first

        const LOW_SIX_BITS: u32 = 0x3F;

        let rem = input.len() % Naive::ENCODE_INPUT_CHUNK_SIZE;
        // will never underflow
        let complete_chunk_len = input.len() - rem;

        let mut input_index = 0_usize;
        let mut output_index = 0_usize;
        if let Some(last_complete_chunk_index) =
            complete_chunk_len.checked_sub(Naive::ENCODE_INPUT_CHUNK_SIZE)
        {
            while input_index <= last_complete_chunk_index {
                let chunk = &input[input_index..input_index + Naive::ENCODE_INPUT_CHUNK_SIZE];

                // populate low 24 bits from 3 bytes
                let chunk_int: u32 =
                    (chunk[0] as u32).shl(16) | (chunk[1] as u32).shl(8) | (chunk[2] as u32);
                // encode 4x 6-bit output bytes
                output[output_index] = self.encode_table[chunk_int.shr(18) as usize];
                output[output_index + 1] =
                    self.encode_table[chunk_int.shr(12_u8).bitand(LOW_SIX_BITS) as usize];
                output[output_index + 2] =
                    self.encode_table[chunk_int.shr(6_u8).bitand(LOW_SIX_BITS) as usize];
                output[output_index + 3] =
                    self.encode_table[chunk_int.bitand(LOW_SIX_BITS) as usize];

                input_index += Naive::ENCODE_INPUT_CHUNK_SIZE;
                output_index += 4;
            }
        }

        // then leftovers
        if rem == 2 {
            let chunk = &input[input_index..input_index + 2];

            // high six bits of chunk[0]
            output[output_index] = self.encode_table[chunk[0].shr(2) as usize];
            // bottom 2 bits of [0], high 4 bits of [1]
            output[output_index + 1] =
                self.encode_table[(chunk[0].shl(4_u8).bitor(chunk[1].shr(4_u8)) as u32)
                    .bitand(LOW_SIX_BITS) as usize];
            // bottom 4 bits of [1], with the 2 bottom bits as zero
            output[output_index + 2] =
                self.encode_table[(chunk[1].shl(2_u8) as u32).bitand(LOW_SIX_BITS) as usize];

            output_index += 3;
        } else if rem == 1 {
            let byte = input[input_index];
            output[output_index] = self.encode_table[byte.shr(2) as usize];
            output[output_index + 1] =
                self.encode_table[(byte.shl(4_u8) as u32).bitand(LOW_SIX_BITS) as usize];
            output_index += 2;
        }

        output_index
    }

    fn decoded_length_estimate(&self, input_len: usize) -> Self::DecodeEstimate {
        NaiveEstimate::from(input_len)
    }

    fn decode(
        &self,
        input: &[u8],
        output: &mut [u8],
        estimate: Self::DecodeEstimate,
    ) -> Result<usize, DecodeError> {
        match estimate.rem {
            1 => {
                // trailing whitespace is so common that it's worth it to check the last byte to
                // possibly return a better error message
                if let Some(b) = input.last() {
                    if *b != PAD_BYTE
                        && self.decode_table[*b as usize] == fast_portable::INVALID_VALUE
                    {
                        return Err(DecodeError::InvalidByte(input.len() - 1, *b));
                    }
                }

                return Err(DecodeError::InvalidLength);
            }
            _ => {}
        }

        let mut input_index = 0_usize;
        let mut output_index = 0_usize;
        const BOTTOM_BYTE: u32 = 0xFF;

        // can only use the main loop on non-trailing chunks
        if input.len() > Naive::DECODE_INPUT_CHUNK_SIZE {
            // skip the last chunk, whether it's partial or full, since it might
            // have padding, and start at the beginning of the chunk before that
            let last_complete_chunk_start_index = estimate.complete_chunk_len
                - if estimate.rem == 0 {
                    // Trailing chunk is also full chunk, so there must be at least 2 chunks, and
                    // this won't underflow
                    Naive::DECODE_INPUT_CHUNK_SIZE * 2
                } else {
                    // Trailing chunk is partial, so it's already excluded in
                    // complete_chunk_len
                    Naive::DECODE_INPUT_CHUNK_SIZE
                };

            while input_index <= last_complete_chunk_start_index {
                let chunk = &input[input_index..input_index + Naive::DECODE_INPUT_CHUNK_SIZE];
                let decoded_int: u32 = self.decode_byte_into_u32(input_index, chunk[0])?.shl(18)
                    | self
                        .decode_byte_into_u32(input_index + 1, chunk[1])?
                        .shl(12)
                    | self.decode_byte_into_u32(input_index + 2, chunk[2])?.shl(6)
                    | self.decode_byte_into_u32(input_index + 3, chunk[3])?;

                output[output_index] = decoded_int.shr(16_u8).bitand(BOTTOM_BYTE) as u8;
                output[output_index + 1] = decoded_int.shr(8_u8).bitand(BOTTOM_BYTE) as u8;
                output[output_index + 2] = decoded_int.bitand(BOTTOM_BYTE) as u8;

                input_index += Naive::DECODE_INPUT_CHUNK_SIZE;
                output_index += 3;
            }
        }

        // handle incomplete chunk -- simplified version of FastPortable
        let mut leftover_bits: u32 = 0;
        let mut morsels_in_leftover = 0;
        let mut padding_bytes = 0;
        let mut first_padding_index: usize = 0;
        let mut last_symbol = 0_u8;
        let start_of_leftovers = input_index;
        for (index, byte) in input[start_of_leftovers..].iter().enumerate() {
            // '=' padding
            if *byte == PAD_BYTE {
                // There can be bad padding in a few ways:
                // 1 - Padding with non-padding characters after it
                // 2 - Padding after zero or one non-padding characters before it
                //     in the current quad.
                // 3 - More than two characters of padding. If 3 or 4 padding chars
                //     are in the same quad, that implies it will be caught by #2.
                //     If it spreads from one quad to another, it will be an invalid byte
                //     in the first quad.

                if index < 2 {
                    // Check for case #2.
                    let bad_padding_index = start_of_leftovers
                        + if padding_bytes > 0 {
                            // If we've already seen padding, report the first padding index.
                            // This is to be consistent with the faster logic above: it will report an
                            // error on the first padding character (since it doesn't expect to see
                            // anything but actual encoded data).
                            first_padding_index
                        } else {
                            // haven't seen padding before, just use where we are now
                            index
                        };
                    return Err(DecodeError::InvalidByte(bad_padding_index, *byte));
                }

                if padding_bytes == 0 {
                    first_padding_index = index;
                }

                padding_bytes += 1;
                continue;
            }

            // Check for case #1.
            // To make '=' handling consistent with the main loop, don't allow
            // non-suffix '=' in trailing chunk either. Report error as first
            // erroneous padding.
            if padding_bytes > 0 {
                return Err(DecodeError::InvalidByte(
                    start_of_leftovers + first_padding_index,
                    PAD_BYTE,
                ));
            }
            last_symbol = *byte;

            // can use up to 4 * 6 = 24 bits of the u32, if last chunk has no padding.
            // Pack the leftovers from left to right.
            let shift = 32 - (morsels_in_leftover + 1) * 6;
            let morsel = self.decode_table[*byte as usize];
            if morsel == fast_portable::INVALID_VALUE {
                return Err(DecodeError::InvalidByte(start_of_leftovers + index, *byte));
            }

            leftover_bits |= (morsel as u32) << shift;
            morsels_in_leftover += 1;
        }

        let leftover_bits_ready_to_append = match morsels_in_leftover {
            0 => 0,
            2 => 8,
            3 => 16,
            4 => 24,
            _ => unreachable!(
                "Impossible: must only have 0 to 4 input bytes in last chunk, with no invalid lengths"
            ),
        };

        // if there are bits set outside the bits we care about, last symbol encodes trailing
        // bits that will not be included in the output
        let mask = !0 >> leftover_bits_ready_to_append;
        if !self.config.decode_allow_trailing_bits && (leftover_bits & mask) != 0 {
            // last morsel is at `morsels_in_leftover` - 1
            return Err(DecodeError::InvalidLastSymbol(
                start_of_leftovers + morsels_in_leftover - 1,
                last_symbol,
            ));
        }

        let mut leftover_bits_appended_to_buf = 0;
        while leftover_bits_appended_to_buf < leftover_bits_ready_to_append {
            // `as` simply truncates the higher bits, which is what we want here
            let selected_bits = (leftover_bits >> (24 - leftover_bits_appended_to_buf)) as u8;
            output[output_index] = selected_bits;
            output_index += 1;

            leftover_bits_appended_to_buf += 8;
        }

        Ok(output_index)
    }

    fn config(&self) -> Self::Config {
        self.config
    }
}

pub struct NaiveEstimate {
    /// remainder from dividing input by `Naive::DECODE_CHUNK_SIZE`
    rem: usize,
    /// Number of complete `Naive::DECODE_CHUNK_SIZE`-length chunks
    complete_chunk_len: usize,
}

impl NaiveEstimate {
    fn from(input_len: usize) -> NaiveEstimate {
        let rem = input_len % Naive::DECODE_INPUT_CHUNK_SIZE;
        let complete_chunk_len = input_len - rem;

        NaiveEstimate {
            rem,
            complete_chunk_len,
        }
    }
}

impl DecodeEstimate for NaiveEstimate {
    fn decoded_length_estimate(&self) -> usize {
        (self.complete_chunk_len + 1) * 3
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NaiveConfig {
    pub padding: bool,
    pub decode_allow_trailing_bits: bool,
}

impl Config for NaiveConfig {
    fn padding(&self) -> bool {
        self.padding
    }
}
