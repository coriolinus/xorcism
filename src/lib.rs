use std::borrow::Borrow;
use std::io::{self, Write};

/// A munger which XORs a key with some data
///
/// This is a low-level structure; more often, you'll want to use [`Writer`], [`Reader`], or [`munge`].
#[derive(Clone)]
pub struct Xorcism<'a> {
    key: &'a [u8],
    pos: usize,
}

impl<'a> Xorcism<'a> {
    /// Create a new Xorcism munger from a key
    pub fn new(key: &'a [u8]) -> Xorcism<'a> {
        Xorcism { key, pos: 0 }
    }

    fn incr_pos(&mut self, by: usize) -> usize {
        let old_pos = self.pos;
        self.pos += by;
        old_pos
    }

    /// XOR each byte of the data with a byte from the key
    pub fn munge<Data, B>(&'a mut self, data: Data) -> impl 'a + Iterator<Item = u8>
    where
        Data: 'a + IntoIterator<Item = B>,
        <Data as IntoIterator>::IntoIter: ExactSizeIterator,
        B: Borrow<u8>,
    {
        let data = data.into_iter();
        let pos = self.incr_pos(data.len());
        data.zip(self.key.iter().cycle().skip(pos))
            .map(|(d, k)| d.borrow() ^ k)
    }

    /// Convert this into a [`Writer`]
    pub fn writer<W>(self, writer: W) -> Writer<'a, W>
    where
        W: Write,
    {
        Writer {
            xorcism: self,
            writer,
        }
    }
}

/// XOR each byte of `key` with each byte of `data`, looping `key` as required.
pub fn munge<Key, Data>(key: Key, data: Data) -> Vec<u8>
where
    Key: AsRef<[u8]>,
    Data: AsRef<[u8]>,
{
    let key = key.as_ref();
    let data = data.as_ref();

    let mut xorcism = Xorcism::new(key);
    xorcism.munge(data).collect()
}

/// This implements `Write` and performs xor munging on the data stream.
///
/// It is constructed with [`Xorcism::writer`].
///
/// It does not perform any internal buffering.
#[derive(Clone)]
pub struct Writer<'a, W> {
    xorcism: Xorcism<'a>,
    writer: W,
}

impl<'a, W> Write for Writer<'a, W>
where
    W: Write,
{
    /// This implementation will block until the underlying writer
    /// has written the entire input buffer.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let munged = {
            let mut munged = Vec::with_capacity(buf.len());
            // rustc can't figure out an appropriate lifetime if we use self.xorcism.munge,
            // so let's work around the issue by reimplementing it. Luckily, it's fairly straightforward.
            let pos = self.xorcism.incr_pos(buf.len());

            munged.extend(
                self.xorcism
                    .key
                    .iter()
                    .cycle()
                    .skip(pos)
                    .zip(buf)
                    .map(|(k, b)| k ^ b),
            );
            munged
        };

        self.writer.write_all(&munged)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity() {
        let mut xs = Xorcism::new(&[0]);
        let data = "This is super-secret, cutting edge encryption, guys.";

        assert_eq!(
            xs.munge(data.as_bytes()).collect::<Vec<_>>(),
            data.as_bytes()
        );
    }

    #[test]
    fn basic_round_trip() {
        let mut xs = Xorcism::new("forsooth, let us never break our trust!".as_bytes());
        let data = "the sacred brothership in which we share will never from our hearts be lost.";

        let mut xs2 = xs.clone();
        let intermediate: Vec<_> = xs.munge(data.as_bytes()).collect();

        assert_ne!(intermediate, data.as_bytes());
        assert_eq!(xs2.munge(intermediate).collect::<Vec<_>>(), data.as_bytes());
    }

    #[test]
    fn writer_roundtrip() {
        let data =
            "Spiderman! It's spiderman! Not a bird, or a plane, or a fireman! Just spiderman!";
        let mut writer_dest = Vec::new();
        let xs1 = Xorcism::new("Who knows what evil lurks in the hearts of men?".as_bytes());
        let xs2 = xs1.clone();
        {
            let mut writer = xs1.writer(xs2.writer(&mut writer_dest));
            assert!(writer.write_all(data.as_bytes()).is_ok());
        }

        assert_eq!(writer_dest, data.as_bytes());
    }

    #[test]
    fn writer_munges() {
        let data = "If wishes were horses, beggars would ride.";
        let mut writer_dest = Vec::new();
        {
            let mut writer =
                Xorcism::new("TRANSMUTATION_NOTES_1".as_bytes()).writer(&mut writer_dest);
            assert!(writer.write_all(data.as_bytes()).is_ok());
        }

        assert_eq!(writer_dest.len(), data.len());
        assert_ne!(writer_dest, data.as_bytes());
    }
}
