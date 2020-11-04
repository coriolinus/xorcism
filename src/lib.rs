use std::borrow::Borrow;
use std::iter::Cycle;
use std::slice::Iter;

/// A munger which XORs a key with some data
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
