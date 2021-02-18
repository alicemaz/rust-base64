use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process;
use std::str::FromStr;

use base64::{alphabet, engine, read, write};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
enum Alphabet {
    Standard,
    UrlSafe,
}

impl Default for Alphabet {
    fn default() -> Self {
        Alphabet::Standard
    }
}

impl FromStr for Alphabet {
    type Err = String;
    fn from_str(s: &str) -> Result<Alphabet, String> {
        match s {
            "standard" => Ok(Alphabet::Standard),
            "urlsafe" => Ok(Alphabet::UrlSafe),
            _ => Err(format!("alphabet '{}' unrecognized", s)),
        }
    }
}

/// Base64 encode or decode FILE (or standard input), to standard output.
#[derive(Debug, StructOpt)]
struct Opt {
    /// decode data
    #[structopt(short = "d", long = "decode")]
    decode: bool,
    /// The alphabet to choose. Defaults to the standard base64 alphabet.
    /// Supported alphabets include "standard" and "urlsafe".
    #[structopt(long = "alphabet")]
    alphabet: Option<Alphabet>,
    /// The file to encode/decode.
    #[structopt(parse(from_os_str))]
    file: Option<PathBuf>,
}

fn main() {
    let opt = Opt::from_args();
    let stdin;
    let mut input: Box<dyn Read> = match opt.file {
        None => {
            stdin = io::stdin();
            Box::new(stdin.lock())
        }
        Some(ref f) if f.as_os_str() == "-" => {
            stdin = io::stdin();
            Box::new(stdin.lock())
        }
        Some(f) => Box::new(File::open(f).unwrap()),
    };

    let alphabet = opt.alphabet.unwrap_or_default();
    let engine = engine::fast_portable::FastPortable::from(
        &match alphabet {
            Alphabet::Standard => alphabet::STANDARD,
            Alphabet::UrlSafe => alphabet::URL_SAFE,
        },
        engine::fast_portable::PAD,
    );

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let r = if opt.decode {
        let mut decoder = read::DecoderReader::from(&mut input, &engine);
        io::copy(&mut decoder, &mut stdout)
    } else {
        let mut encoder = write::EncoderWriter::from(&mut stdout, &engine);
        io::copy(&mut input, &mut encoder)
    };
    if let Err(e) = r {
        eprintln!(
            "Base64 {} failed with {}",
            if opt.decode { "decode" } else { "encode" },
            e
        );
        process::exit(1);
    }
}
