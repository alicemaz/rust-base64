//! Provides the [Engine] abstraction and out of the box implementations.
use crate::engine::fast_portable::FastPortable;
use crate::{alphabet, DecodeError};

pub mod fast_portable;

#[cfg(test)]
mod naive;

#[cfg(test)]
mod tests;

// TODO shared DecodeError or per-impl as an associated type?

/// An `Engine` provides low-level encoding and decoding operations that all other higher-level parts of the API use.
///
/// Different implementations offer different characteristics. The library currently ships with
/// a general-purpose [FastPortable] that offers good speed and works on any CPU, with more choices
/// coming later, like a constant-time one when side channel resistance is called for, and vendor-specific vectorized ones for more speed.
///
/// See [DEFAULT_ENGINE] if you just want standard base64. Otherwise, when possible, it's
/// recommended to store the engine in a `const` so that references to it won't pose any lifetime
/// issues, and to avoid repeating the cost of engine setup.
// When adding an implementation of Engine, include them in the engine test suite:
// - add an implementation of [engine::tests::EngineWrapper]
// - add the implementation to the `all_engines` macro
// All tests run on all engines listed in the macro.
pub trait Engine {
    /// The config type used by this engine
    type Config: Config;
    /// The decode estimate used by this engine
    type DecodeEstimate: DecodeEstimate;

    /// Encode the `input` bytes into the `output` buffer based on the mapping in `encode_table`.
    ///
    /// `output` will be long enough to hold the encoded data.
    ///
    /// Returns the number of bytes written.
    ///
    /// No padding should be written; that is handled separately.
    ///
    /// Must not write any bytes into the output slice other than the encoded data.
    fn encode(&self, input: &[u8], output: &mut [u8]) -> usize;

    /// As an optimization, it is sometimes helpful to have a conservative estimate of the decoded
    /// size before doing the decoding.
    ///
    /// The result of this must be passed to [Engine::decode()].
    fn decoded_length_estimate(&self, input_len: usize) -> Self::DecodeEstimate;

    /// Decode `input` base64 bytes into the `output` buffer.
    ///
    /// `decode_estimate` is the result of [Engine::decoded_length_estimate()], which is passed in to avoid
    /// calculating it again (expensive on short inputs).`
    ///
    /// Returns the number of bytes written to `output`.
    ///
    /// Each complete 4-byte chunk of encoded data decodes to 3 bytes of decoded data, but this
    /// function must also handle the final possibly partial chunk.
    /// If the input length is not a multiple of 4, or uses padding bytes to reach a multiple of 4,
    /// the trailing 2 or 3 bytes must decode to 1 or 2 bytes, respectively, as per the
    /// [RFC](https://tools.ietf.org/html/rfc4648#section-3.5).
    ///
    /// Decoding must not write any bytes into the output slice other than the decoded data.
    fn decode(
        &self,
        input: &[u8],
        output: &mut [u8],
        decode_estimate: Self::DecodeEstimate,
    ) -> Result<usize, DecodeError>;

    /// Returns the config for this engine.
    fn config(&self) -> Self::Config;
}

/// The minimal level of configuration engines must expose.
pub trait Config {
    /// Returns `true` if padding should be added after the encoded output.
    ///
    /// Padding is added outside the engine's encode() since the engine may be used
    /// to encode only a chunk of the overall output, so it can't always know when
    /// the output is "done" and would therefore need padding (if configured).
    // It could be provided as a separate parameter when encoding, but that feels like
    // leaking an implementation detail to the user, and it's hopefully more convenient
    // to have to only pass one thing (the engine) to any part of the API.
    fn padding(&self) -> bool;
}

/// The decode estimate used by an engine implementation.
///
/// Implementors may want to store relevant calculations when constructing this to avoid having
/// to calculate them again during actual decoding.
pub trait DecodeEstimate {
    /// Returns a conservative (err on the side of too big) estimate of the decoded length to use
    /// for pre-allocating buffers, etc.
    fn decoded_length_estimate(&self) -> usize;
}

/// An engine that will work on all CPUs using the standard base64 alphabet.
pub const DEFAULT_ENGINE: FastPortable =
    FastPortable::from(&alphabet::STANDARD, fast_portable::PAD);
