extern crate base64;

use base64::engine::fast_portable::{FastPortable, NO_PAD};
use base64::engine::DEFAULT_ENGINE;
use base64::*;

use self::helpers::*;

mod helpers;

#[test]
fn decode_rfc4648_0() {
    compare_decode("", "");
}

#[test]
fn decode_rfc4648_1() {
    compare_decode("f", "Zg==");
}

#[test]
fn decode_rfc4648_1_just_a_bit_of_padding() {
    // allows less padding than required
    compare_decode("f", "Zg=");
}

#[test]
fn decode_rfc4648_1_no_padding() {
    compare_decode("f", "Zg");
}

#[test]
fn decode_rfc4648_2() {
    compare_decode("fo", "Zm8=");
}

#[test]
fn decode_rfc4648_2_no_padding() {
    compare_decode("fo", "Zm8");
}

#[test]
fn decode_rfc4648_3() {
    compare_decode("foo", "Zm9v");
}

#[test]
fn decode_rfc4648_4() {
    compare_decode("foob", "Zm9vYg==");
}

#[test]
fn decode_rfc4648_4_no_padding() {
    compare_decode("foob", "Zm9vYg");
}

#[test]
fn decode_rfc4648_5() {
    compare_decode("fooba", "Zm9vYmE=");
}

#[test]
fn decode_rfc4648_5_no_padding() {
    compare_decode("fooba", "Zm9vYmE");
}

#[test]
fn decode_rfc4648_6() {
    compare_decode("foobar", "Zm9vYmFy");
}

#[test]
fn decode_reject_null() {
    assert_eq!(
        DecodeError::InvalidByte(3, 0x0),
        decode_engine("YWx\0pY2U==", &DEFAULT_ENGINE).unwrap_err()
    );
}

#[test]
fn decode_imap() {
    assert_eq!(
        decode_engine(b"+,,+", &FastPortable::from(&alphabet::IMAP_MUTF7, NO_PAD),),
        decode_engine(b"+//+", &DEFAULT_ENGINE)
    );
}
