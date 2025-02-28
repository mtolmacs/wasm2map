use crate::error::Error;
use object::{ReadCache, ReadRef};
use std::{
    fs::File,
    io::{Read, Seek},
    path::Path,
};

///
#[derive(Debug)]
pub struct Loader<R: Read + Seek> {
    data: ReadCache<R>,
}

impl Loader<File> {
    ///
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let file = File::open(path)?;
        Ok(Self {
            data: ReadCache::new(file),
        })
    }

    ///
    pub fn from_file(file: File) -> Result<Self, Error> {
        Ok(Self {
            data: ReadCache::new(file),
        })
    }
}

impl<'a, R: Read + Seek> ReadRef<'a> for &'a Loader<R> {
    fn len(self) -> Result<u64, ()> {
        self.data.len()
    }

    fn read_bytes_at(self, offset: u64, size: u64) -> Result<&'a [u8], ()> {
        self.data.read_bytes_at(offset, size)
    }

    fn read_bytes_at_until(
        self,
        range: std::ops::Range<u64>,
        delimiter: u8,
    ) -> Result<&'a [u8], ()> {
        self.data.read_bytes_at_until(range, delimiter)
    }
}
