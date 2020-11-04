use std::borrow::Borrow;
use std::io::{self, Write};
use std::iter::Cycle;
use std::slice::Iter;

/// A munger which XORs a key with some data
///
/// This is a low-level structure; more often, you'll want to use [`Writer`], [`Reader`], or [`munge`].
#[derive(Clone)]
pub struct Xorcism<'a> {
    key: Cycle<Iter<'a, u8>>,
}

impl<'a> Xorcism<'a> {
    /// Create a new Xorcism munger from a key
    pub fn new(key: &'a [u8]) -> Xorcism<'a> {
        Xorcism {
            key: key.into_iter().cycle(),
        }
    }

    /// XOR each byte of the data with a byte from the key
    pub fn munge<Data, B>(&'a mut self, data: Data) -> impl 'a + Iterator<Item = u8>
    where
        Data: 'a + IntoIterator<Item = B>,
        B: Borrow<u8>,
    {
        data.into_iter()
            .zip(self.key.by_ref())
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
        let mut munged = Vec::with_capacity(buf.len());
        munged.extend(self.xorcism.munge(buf));

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
}
