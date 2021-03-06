use std::borrow::Borrow;
use std::io::{self, Read, Write};

/// A munger which XORs a key with some data
///
/// This is a low-level structure; more often, you'll want to use [`Writer`], [`Reader`], or [`munge`].
//
// You might wonder: why implement this manually, instead of just storing `key: Cycle<Iter<'a, u8>>,`?
//
// If we do it like that, the lifetimes get kind of crazy. In particular, in `fn munge`, we want to do
// `data.zip(self.key.by_ref())`, and that `by_ref()` thing really confuses the lifetime inferencer.
// It ended up being simpler to just handle the key indexing manually than to figure out the correct
// incantation to get that lifetime style to work.
#[derive(Clone)]
pub struct Xorcism<'a> {
    key: &'a [u8],
    pos: usize,
}

impl<'a> Xorcism<'a> {
    /// Create a new Xorcism munger from a key
    pub fn new<Key>(key: &'a Key) -> Xorcism<'a>
    where
        Key: AsRef<[u8]> + ?Sized,
    {
        let key = key.as_ref();
        Xorcism { key, pos: 0 }
    }

    /// Increase the stored pos by the specified amount, returning the old value.
    fn incr_pos(&mut self, by: usize) -> usize {
        let old_pos = self.pos;
        self.pos += by;
        old_pos
    }

    /// Produce the key iterator, offset by `pos`.
    fn key<'b>(&mut self, pos: usize) -> impl 'b + Iterator<Item = u8>
    where
        'a: 'b,
    {
        self.key.iter().copied().cycle().skip(pos)
    }

    /// XOR each byte of the input buffer with a byte from the key.
    ///
    /// Note that this is stateful: repeated calls are likely to produce different results,
    /// even with identical inputs.
    pub fn munge_in_place(&mut self, data: &mut [u8]) {
        let pos = self.incr_pos(data.len());
        for (d, k) in data.iter_mut().zip(self.key(pos)) {
            *d ^= k;
        }
    }

    /// XOR each byte of the data with a byte from the key.
    ///
    /// Note that this is stateful: repeated calls are likely to produce different results,
    /// even with identical inputs.
    pub fn munge<'b, Data, B>(&mut self, data: Data) -> impl 'b + Iterator<Item = u8>
    where
        'a: 'b,
        Data: IntoIterator<Item = B>,
        <Data as IntoIterator>::IntoIter: 'b + ExactSizeIterator,
        B: Borrow<u8>,
    {
        let data = data.into_iter();
        let pos = self.incr_pos(data.len());
        data.zip(self.key(pos)).map(|(d, k)| d.borrow() ^ k)
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

    /// Convert this into a [`Reader`]
    pub fn reader<R>(self, reader: R) -> Reader<'a, R>
    where
        R: Read,
    {
        Reader {
            xorcism: self,
            reader,
        }
    }
}

/// XOR each byte of `key` with each byte of `data`, looping `key` as required.
///
/// This is stateless: repeated calls with identical inputs will always produce identical results.
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
#[derive(Clone)]
pub struct Writer<'a, W> {
    xorcism: Xorcism<'a>,
    writer: W,
}

impl<'a, W> Writer<'a, W>
where
    W: Write,
{
    pub fn new<Key>(key: &'a Key, writer: W) -> Writer<'a, W>
    where
        Key: AsRef<[u8]> + ?Sized,
    {
        Writer {
            xorcism: Xorcism::new(key),
            writer,
        }
    }
}

impl<'a, W> Write for Writer<'a, W>
where
    W: Write,
{
    /// This implementation will block until the underlying writer
    /// has written the entire input buffer.
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let munged: Vec<_> = self.xorcism.munge(data).collect();
        self.writer.write_all(&munged)?;
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

/// This implements `Read` and performs xor munging on the data stream.
#[derive(Clone)]
pub struct Reader<'a, R> {
    xorcism: Xorcism<'a>,
    reader: R,
}

impl<'a, R> Reader<'a, R>
where
    R: Read,
{
    pub fn new<Key>(key: &'a Key, reader: R) -> Reader<'a, R>
    where
        Key: AsRef<[u8]> + ?Sized,
    {
        Reader {
            xorcism: Xorcism::new(key),
            reader,
        }
    }
}

impl<'a, R> Read for Reader<'a, R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let bytes_read = self.reader.read(buf)?;
        self.xorcism.munge_in_place(&mut buf[..bytes_read]);
        Ok(bytes_read)
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
        let mut xs = Xorcism::new("forsooth, let us never break our trust!");
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
        let xs1 = Xorcism::new("Who knows what evil lurks in the hearts of men?");
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
            let mut writer = Writer::new("TRANSMUTATION_NOTES_1", &mut writer_dest);
            assert!(writer.write_all(data.as_bytes()).is_ok());
        }

        assert_eq!(writer_dest.len(), data.len());
        assert_ne!(writer_dest, data.as_bytes());
    }

    #[test]
    fn reader_munges() {
        let data = "The globe is text, its people prose; all the world's a page.";
        let mut reader = Reader::new("But who owns the book?", data.as_bytes());
        let mut buf = Vec::with_capacity(data.len());
        let bytes_read = reader.read_to_end(&mut buf).unwrap();
        assert_eq!(bytes_read, data.len());
        assert_ne!(buf, data.as_bytes());
    }

    #[test]
    fn reader_roundtrip() {
        let data = "Mary Poppins was a kind witch. She cared for the children.";
        let key = "supercalifragilisticexpialidocious.";
        let mut reader = Reader::new(key, Reader::new(key, data.as_bytes()));
        let mut buf = Vec::with_capacity(data.len());
        let bytes_read = reader.read_to_end(&mut buf).unwrap();
        assert_eq!(bytes_read, data.len());
        assert_eq!(buf, data.as_bytes());
    }
}
