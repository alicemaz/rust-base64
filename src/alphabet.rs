//! Provides [Alphabet] and constants for alphabets commonly used in the wild.

/// An alphabet defines the 64 ASCII characters (symbols) used for base64.
///
/// Common alphabets are provided as constants, and custom alphabets
/// can be made via the [From](#impl-From<T>) implementation.
///
/// ```
/// let custom = base64::alphabet::Alphabet::from("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/");
///
/// let engine = base64::engine::fast_portable::FastPortable::from(
///     &custom,
///     base64::engine::fast_portable::PAD);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Alphabet {
    pub(crate) symbols: [u8; 64],
}

impl Alphabet {
    /// Performs no checks so that it can be const.
    /// Used only for known-valid strings.
    const fn from_unchecked(alphabet: &str) -> Alphabet {
        let mut symbols = [0_u8; 64];
        let source_bytes = alphabet.as_bytes();

        // a way to copy that's allowed in const fn
        let mut index = 0;
        while index < 64 {
            symbols[index] = source_bytes[index];
            index += 1;
        }

        Alphabet { symbols }
    }
}

impl<T: AsRef<str>> From<T> for Alphabet {
    /// Create a `CharacterSet` from a string of 64 ASCII bytes. Each byte must be
    /// unique, and the `=` byte is not allowed as it is used for padding.
    ///
    /// # Errors
    ///
    /// Panics if the text is an invalid base64 alphabet since the alphabet is
    /// likely to be hardcoded, and therefore errors are generally unrecoverable
    /// programmer errors.
    fn from(string: T) -> Self {
        let alphabet = string.as_ref();
        assert_eq!(
            64,
            alphabet.as_bytes().len(),
            "Base64 char set length must be 64"
        );

        // scope just to ensure not accidentally using the sorted copy
        {
            // Check uniqueness without allocating since this must be no_std.
            // Could pull in heapless and use IndexSet, but this seems simple enough.
            let mut bytes = [0_u8; 64];
            alphabet
                .as_bytes()
                .iter()
                .enumerate()
                .for_each(|(index, &byte)| bytes[index] = byte);

            bytes.sort_unstable();

            // iterate over the sorted bytes, offset by one
            bytes.iter().zip(bytes[1..].iter()).for_each(|(b1, b2)| {
                // if any byte is the same as the next byte, there's a duplicate
                assert_ne!(b1, b2, "Duplicate bytes");
            });
        }

        for &byte in alphabet.as_bytes() {
            // must be ascii printable. 127 (DEL) is commonly considered printable
            // for some reason but clearly unsuitable for base64.
            assert!(byte >= 32_u8 && byte < 127_u8, "Bytes must be printable");
            // = is assumed to be padding, so cannot be used as a symbol
            assert_ne!(b'=', byte, "Padding byte '=' is reserved");
        }

        Self::from_unchecked(alphabet)
    }
}

/// The standard alphabet (uses `+` and `/`).
///
/// See [RFC 3548](https://tools.ietf.org/html/rfc3548#section-3).
pub const STANDARD: Alphabet =
    Alphabet::from_unchecked("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/");

/// The URL safe alphabet (uses `-` and `_`).
///
/// See [RFC 3548](https://tools.ietf.org/html/rfc3548#section-4).
pub const URL_SAFE: Alphabet =
    Alphabet::from_unchecked("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_");

/// The `crypt(3)` alphabet (uses `.` and `/` as the first two values).
///
/// Not standardized, but folk wisdom on the net asserts that this alphabet is what crypt uses.
pub const CRYPT: Alphabet =
    Alphabet::from_unchecked("./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz");

/// The bcrypt alphabet.
pub const BCRYPT: Alphabet =
    Alphabet::from_unchecked("./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789");

/// The alphabet used in IMAP-modified UTF-7 (uses `+` and `,`).
///
/// See [RFC 3501](https://tools.ietf.org/html/rfc3501#section-5.1.3)
pub const IMAP_MUTF7: Alphabet =
    Alphabet::from_unchecked("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+,");

/// The alphabet used in BinHex 4.0 files.
///
/// See [BinHex 4.0 Definition](http://files.stairways.com/other/binhex-40-specs-info.txt)
pub const BIN_HEX: Alphabet =
    Alphabet::from_unchecked("!\"#$%&'()*+,-0123456789@ABCDEFGHIJKLMNPQRSTUVXYZ[`abcdehijklmpqr");

#[cfg(test)]
mod tests {
    use crate::alphabet::Alphabet;

    #[should_panic(expected = "Duplicate bytes")]
    #[test]
    fn detects_duplicate_start() {
        let _ = Alphabet::from("AACDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/");
    }

    #[should_panic(expected = "Duplicate bytes")]
    #[test]
    fn detects_duplicate_end() {
        let _ = Alphabet::from("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789//");
    }

    #[should_panic(expected = "Duplicate bytes")]
    #[test]
    fn detects_duplicate_middle() {
        let _ = Alphabet::from("ABCDEFGHIJKLMNOPQRSTUVWXYZZbcdefghijklmnopqrstuvwxyz0123456789+/");
    }

    #[should_panic(expected = "Base64 char set length must be 64")]
    #[test]
    fn detects_length() {
        let _ = Alphabet::from(
            "xxxxxxxxxABCDEFGHIJKLMNOPQRSTUVWXYZZbcdefghijklmnopqrstuvwxyz0123456789+/",
        );
    }

    #[should_panic(expected = "Padding byte '=' is reserved")]
    #[test]
    fn detects_padding() {
        let _ = Alphabet::from("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+=");
    }

    #[should_panic(expected = "Bytes must be printable")]
    #[test]
    fn detects_unprintable() {
        // form feed
        let _ =
            Alphabet::from("\x0cBCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/");
    }
}
