//! Implementations of `io::Write` to transparently handle base64.
mod encoder;
mod encoder_string_writer;
pub use self::encoder::EncoderWriter;
pub use self::encoder_string_writer::EncoderStringWriter;
pub use self::encoder_string_writer::StrConsumer;

#[cfg(test)]
mod encoder_tests;
