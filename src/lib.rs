//!
//! # Alphabets
//!
//! An [alphabet::Alphabet] defines what ASCII symbols are used to encode to or decode from.
//!
//! Constants in [alphabet] like [alphabet::STANDARD] or [alphabet::URL_SAFE] provide commonly used
//! alphabets, but you can also build your own custom `Alphabet` if needed.
//!
//! # Engines
//!
//! Once you have an `Alphabet`, you can pick which `Engine` you want. A few parts of the public
//! API provide a default, but otherwise the user must provide an `Engine` to use.
//!
//! See [engine::Engine] for more on what engine to choose, or use [engine::DEFAULT_ENGINE] if you
//! just want plain old standard base64 and don't have other requirements.
//!
//! ## Config
//!
//! In addition to an `Alphabet`, constructing an `Engine` also requires an [engine::Config]. Each
//! `Engine` has a corresponding `Config` implementation.
//!
//! [encode()] and [decode()] use the standard alphabet and default engine in an RFC 4648 standard
//! setup.
//!
//! # Encoding
//!
//! Several different encoding functions are available to you depending on your desire for
//! convenience vs performance.
//!
//! | Function                | Output                       | Allocates                      |
//! | ----------------------- | ---------------------------- | ------------------------------ |
//! | `encode`                | Returns a new `String`       | Always                         |
//! | `encode_engine`         | Returns a new `String`       | Always                         |
//! | `encode_engine_string`     | Appends to provided `String` | Only if `String` needs to grow |
//! | `encode_engine_slice`   | Writes to provided `&[u8]`   | Never - fastest                |
//!
//! All of the encoding functions that take an `Engine` will pad as per the engine's config.
//!
//! # Decoding
//!
//! Just as for encoding, there are different decoding functions available.
//!
//! | Function                | Output                        | Allocates                      |
//! | ----------------------- | ----------------------------- | ------------------------------ |
//! | `decode`                | Returns a new `Vec<u8>`       | Always                         |
//! | `decode_engine`         | Returns a new `Vec<u8>`       | Always                         |
//! | `decode_engine_vec`     | Appends to provided `Vec<u8>` | Only if `Vec` needs to grow    |
//! | `decode_engine_slice`   | Writes to provided `&[u8]`    | Never - fastest                |
//!
//! Unlike encoding, where all possible input is valid, decoding can fail (see [DecodeError]).
//!
//! Input can be invalid because it has invalid characters or invalid padding. (No padding at all is
//! valid, but excess padding is not.) Whitespace in the input is invalid, just like any other
//! non-base64 byte.
//!
//! # `Read` and `Write`
//!
//! To decode a [std::io::Read] of b64 bytes, wrap a reader (file, network socket, etc) with
//! [read::DecoderReader].
//!
//! To write raw bytes and have them b64 encoded on the fly, wrap a [std::io::Write] with
//! [write::EncoderWriter].
//!
//! There is some performance overhead (15% or so) because of the necessary buffer shuffling --
//! still fast enough that almost nobody cares. Also, these implementations do not heap allocate.
//!
//! # `Display`
//!
//! See [display] for how to transparently base64 data via a `Display` implementation.
//!
//! # Panics
//!
//! If length calculations result in overflowing `usize`, a panic will result.
//!
//! The `_slice` flavors of encode or decode will panic if the provided output slice is too small.

#![cfg_attr(feature = "cargo-clippy", allow(clippy::cast_lossless))]
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_results,
    variant_size_differences,
    warnings
)]
#![forbid(unsafe_code)]
#![cfg_attr(not(any(feature = "std", test)), no_std)]

#[cfg(all(feature = "alloc", not(any(feature = "std", test))))]
extern crate alloc;
#[cfg(any(feature = "std", test))]
extern crate std as alloc;

// has to be included at top level because of the way rstest_reuse defines its macros
#[cfg(test)]
use rstest_reuse;

mod chunked_encoder;
pub mod display;
#[cfg(any(feature = "std", test))]
pub mod read;
#[cfg(any(feature = "std", test))]
pub mod write;

pub mod engine;

pub mod alphabet;

mod encode;
pub use crate::encode::encode_engine_slice;
#[cfg(any(feature = "alloc", feature = "std", test))]
pub use crate::encode::{encode, encode_engine, encode_engine_string};

mod decode;
#[cfg(any(feature = "alloc", feature = "std", test))]
pub use crate::decode::{decode, decode_engine, decode_engine_vec};
pub use crate::decode::{decode_engine_slice, DecodeError};

#[cfg(test)]
mod tests;

const PAD_BYTE: u8 = b'=';
