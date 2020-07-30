use crate::{
    pack,
    zlib::stream::{inflate::Inflate, InflateReader},
};
use git_object::owned;
use quick_error::quick_error;
use std::{fs, io};

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(err: io::Error) {
            display("An IO operation failed while streaming an entry")
            from()
            source(err)
        }
        PackParse(err: pack::data::parse::Error) {
            display("The pack header could not be parsed")
            from()
            source(err)
        }
    }
}

#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde1", derive(serde::Serialize, serde::Deserialize))]
pub struct Entry {
    pub header: pack::data::Header,
    /// amount of bytes used to encode the `header`. `pack_offset + header_size` is the beginning of the compressed data in the pack.
    pub header_size: u16,
    pub pack_offset: u64,
    /// amount bytes consumed while producing `decompressed`
    pub compressed: Vec<u8>,
    /// The decompressed data.
    pub decompressed: Vec<u8>,
}

pub struct Iter<R> {
    read: R,
    decompressor: Option<Inflate>,
    offset: u64,
    had_error: bool,
    kind: pack::data::Kind,
    objects_left: u32,
    hash: Option<owned::Id>,
    verify: bool,
}

impl<R> Iter<R>
where
    R: io::BufRead,
{
    /// Note that `read` is expected at the beginning of a valid pack file with header and trailer
    /// If `verify` is true, we will assert the SHA1 is actually correct before returning the last entry.
    /// Otherwise bit there is a chance that some kinds of bitrot or inconsistencies will not be detected.
    pub fn new_from_header(mut read: R, verify: bool) -> Result<Iter<R>, Error> {
        let mut header_data = [0u8; 12];
        read.read_exact(&mut header_data)?;

        let (kind, num_objects) = pack::data::parse::header(&header_data)?;
        assert_eq!(
            kind,
            pack::data::Kind::V2,
            "let's stop here if we see undocumented pack formats"
        );
        Ok(Iter {
            read,
            decompressor: None,
            offset: 12,
            had_error: false,
            kind,
            objects_left: num_objects,
            hash: None,
            verify,
        })
    }

    pub fn kind(&self) -> pack::data::Kind {
        self.kind
    }

    /// Can only be queried once the iterator has been exhausted and `len()` returns 0
    pub fn checksum(&self) -> owned::Id {
        self.hash.expect("iterator must be exhausted")
    }

    fn next_inner(&mut self) -> Result<Entry, Error> {
        let (header, decompressed_size, header_size) =
            pack::data::Header::from_read(&mut self.read, self.offset).map_err(Error::from)?;

        let mut decompressor = self.decompressor.take().unwrap_or_default();
        decompressor.reset();
        let mut reader = InflateReader {
            inner: PassThrough {
                read: &mut self.read,
                write: Vec::with_capacity((decompressed_size / 2) as usize),
            },
            decompressor,
        };

        let mut decompressed = Vec::with_capacity(decompressed_size as usize);
        let bytes_copied = io::copy(&mut reader, &mut decompressed)?;

        assert_eq!(
            bytes_copied, decompressed_size,
            "We should have decompressed {} bytes, but got {} instead",
            decompressed_size, bytes_copied
        );

        let pack_offset = self.offset;
        let compressed_size = reader.decompressor.total_in;
        self.offset += header_size as u64 + compressed_size;
        self.decompressor = Some(reader.decompressor);
        let mut compressed = reader.inner.write;
        compressed.shrink_to_fit();
        assert_eq!(
            compressed_size,
            compressed.len() as u64,
            "we must track exactly the same amount of bytes as read by the decompressor"
        );

        Ok(Entry {
            header,
            // TODO: remove this field once we can pack-encode the header above
            header_size: header_size as u16,
            compressed,
            pack_offset,
            decompressed,
        })
    }
}

impl<R> Iterator for Iter<R>
where
    R: io::BufRead,
{
    type Item = Result<Entry, Error>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.objects_left as usize, Some(self.objects_left as usize))
    }

    fn next(&mut self) -> Option<Self::Item> {
        if self.had_error || self.objects_left == 0 {
            return None;
        }
        self.objects_left -= 1; // even an error counts as objects
        let result = self.next_inner();
        self.had_error = result.is_err();
        Some(result)
    }
}
impl<R> std::iter::ExactSizeIterator for Iter<R> where R: io::BufRead {}

struct PassThrough<R, W> {
    read: R,
    write: W,
}

impl<R, W> io::BufRead for PassThrough<R, W>
where
    Self: io::Read,
    R: io::BufRead,
    W: io::Write,
{
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.read.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        let buf = self
            .read
            .fill_buf()
            .expect("never fail as we called fill-buf before and this does nothing");
        self.write
            .write_all(&buf[..amt])
            .expect("a write to never fail - should be a memory buffer");
        self.read.consume(amt)
    }
}

impl<R, W> io::Read for PassThrough<R, W>
where
    R: io::Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read.read(buf)
    }
}

impl pack::data::File {
    /// Returns an iterator over the pack file itself, without making use of the memory mapping.
    ///
    /// Note that this iterator is costly as no pack index is used, forcing each entry to be decompressed.
    /// If an index is available, use the `traverse(…)` method instead for maximum performance.
    pub fn iter(&self) -> Result<Iter<io::BufReader<fs::File>>, Error> {
        let reader = io::BufReader::new(fs::File::open(&self.path)?);
        Iter::new_from_header(reader, false)
    }
}