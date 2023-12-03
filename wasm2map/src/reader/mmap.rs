use std::{
    fs::File,
    io::{Read, Seek},
    marker::PhantomData,
    path::Path,
};

use memmap2::Mmap;
use object::ReadRef;

use crate::error::Error;

///
#[derive(Debug)]
pub struct WasmReader<R: Read + Seek> {
    data: Mmap,
    _marker: PhantomData<R>,
}

impl WasmReader<File> {
    ///
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, Error> {
        let file = File::open(path)?;

        Ok(Self {
            data: unsafe { memmap2::Mmap::map(&file) }?,
            _marker: PhantomData,
        })
    }

    ///
    pub fn from_file(file: File) -> Result<Self, Error> {
        Ok(Self {
            data: unsafe { memmap2::Mmap::map(&file) }?,
            _marker: PhantomData,
        })
    }
}

impl<'a, R: Read + Seek> ReadRef<'a> for &'a WasmReader<R> {
    fn len(self) -> Result<u64, ()> {
        self.data.len().try_into().map_err(|_| ())
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
