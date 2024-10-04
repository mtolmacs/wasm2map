use std::borrow::Cow;

/// We need a holder struct to own the binary data coming out of the object
/// reader when the DWARF loader loads a section. Since the gimli::Reader trait
/// is not implemented for Cow returned by object::File::section_by_name we
/// need to implement it ourselves.
#[derive(Clone, Debug)]
pub struct SectionReader<'a> {
    pub data: Cow<'a, [u8]>,
}

impl<'a> std::ops::Deref for SectionReader<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.data.deref()
    }
}

unsafe impl<'a> gimli::StableDeref for SectionReader<'a> {}
unsafe impl<'a> gimli::CloneStableDeref for SectionReader<'a> {}
