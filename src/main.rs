#[cfg(feature = "bin")]
mod main {
    use color_eyre::eyre::{Report, Result};
    use std::path::PathBuf;
    use structopt::StructOpt;

    /// newtype so structopt's type magic doesn't misinterpret our intentions
    #[cfg(unix)]
    #[derive(Debug)]
    struct Bytes(Vec<u8>);

    #[cfg(unix)]
    impl From<&Bytes> for Vec<u8> {
        fn from(Bytes(owned): &Bytes) -> Vec<u8> {
            owned.to_vec()
        }
    }

    #[cfg(unix)]
    fn parse_key_literal(src: &std::ffi::OsStr) -> Bytes {
        use std::os::unix::ffi::OsStrExt;
        Bytes(src.as_bytes().into())
    }

    /// Munge data using XOR
    ///
    /// Data is read from stdin and emitted via stdout:
    ///
    ///   xorcism --key-file key_file < in_file > out_file
    #[derive(Debug, StructOpt)]
    struct Opt {
        /// Key literal
        ///
        /// If set, use this value as a literal key
        #[cfg(unix)]
        #[structopt(
            short = "k",
            long,
            parse(from_os_str = parse_key_literal),
            conflicts_with = "key_file",
            env = "XORCISM_KEY"
        )]
        key: Option<Bytes>,

        /// Key file
        ///
        /// If set, use the contents of this file as the key
        #[structopt(short = "f", long, parse(from_os_str))]
        key_file: Option<PathBuf>,

        /// Use a single null byte as the key (debug only)
        #[cfg(feature = "debug")]
        #[structopt(long)]
        null_key: bool,

        /// Always encode output as base64
        ///
        /// Default: true when stdout is a TTY; false otherwise
        #[structopt(long)]
        base64: bool,

        /// Never encode output as base64
        #[structopt(long, conflicts_with = "base64")]
        no_base64: bool,
    }

    impl Opt {
        /// Encode output as base64
        fn base64(&self) -> bool {
            // it might be possible to code this as a single binary expression using
            // short-circuiting, but this more explicit form was simpler
            if self.base64 {
                true
            } else if self.no_base64 {
                false
            } else {
                atty::is(atty::Stream::Stdout)
            }
        }

        /// Get key
        fn key(&self) -> Result<Vec<u8>> {
            #[cfg(feature = "debug")]
            {
                if self.null_key {
                    return Ok(vec![0]);
                }
            }

            #[cfg(unix)]
            {
                if let Some(bytes) = &self.key {
                    return Ok(bytes.into());
                }
            }

            if let Some(path) = &self.key_file {
                return std::fs::read(path).map_err(Into::into);
            }

            Err(Report::msg("key not provided"))
        }
    }

    pub fn main() -> Result<()> {
        color_eyre::install()?;
        let opt = Opt::from_args();

        {
            let key = opt.key()?;
            if key.len() == 0 {
                Err(Report::msg("key must have size > 0"))?
            }
            let reader = std::io::stdin();
            let mut reader = std::io::BufReader::new(reader.lock());

            let writer = std::io::stdout();
            let writer = std::io::BufWriter::new(writer.lock());
            let writer: Box<dyn std::io::Write> = if opt.base64() {
                Box::new(base64::write::EncoderWriter::new(writer, base64::STANDARD))
            } else {
                Box::new(writer)
            };
            let mut writer = xorcism::Writer::new(&key, writer);

            std::io::copy(&mut reader, &mut writer)?;
        }

        // if stdout is a terminal, emit a trailing newline
        if atty::is(atty::Stream::Stdout) {
            println!();
        }

        Ok(())
    }
}

#[cfg(not(feature = "bin"))]
fn main() {
    let bin_name = std::env::args().next().unwrap_or("xorcism".into());
    let bin_name = bin_name.rsplit('/').next().unwrap_or(&bin_name);
    eprintln!("{} was compiled without the `bin` feature", bin_name);
    std::process::exit(1);
}

#[cfg(feature = "bin")]
fn main() -> color_eyre::eyre::Result<()> {
    main::main()
}
