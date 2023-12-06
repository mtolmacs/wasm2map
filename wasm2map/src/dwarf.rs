use crate::{
    error::Error,
    relocate::{Relocate, RelocationMap},
};
use gimli::{Dwarf, EndianReader, LittleEndian, Reader};
use object::{
    File, Object, ObjectSection, ObjectSymbol, ReadRef, RelocationKind, RelocationTarget, Section,
};
use std::{borrow::Cow, cell::OnceCell, rc::Rc};

pub type Relocator<'a> = Relocate<EndianReader<LittleEndian, SectionReader<'a>>>;

pub struct Raw<'reader, R: ReadRef<'reader>> {
    binary: File<'reader, R>,
    dwo_parent: Option<File<'reader, R>>,
    sup_file: Option<File<'reader, R>>,
}

impl<'reader, R> Raw<'reader, R>
where
    R: ReadRef<'reader> + 'reader,
{
    ///
    pub fn new(binary: R, dwo_parent: Option<R>, sup_file: Option<R>) -> Result<Self, Error> {
        Ok(Self {
            binary: Self::parse_file(binary)?,
            dwo_parent: dwo_parent.and_then(|dwo_parent| Self::parse_file(dwo_parent).ok()),
            sup_file: sup_file.and_then(|sup_file| Self::parse_file(sup_file).ok()),
        })
    }

    ///
    fn parse_file(binary: R) -> Result<File<'reader, R>, Error> {
        match File::parse(binary)? {
            file @ File::Wasm(_) => Ok(file),
            _ => Err(Error::from("Data does not represent a WASM file")),
        }
    }
}

///
///
///
pub struct DwarfReader<'reader, R: ReadRef<'reader> + 'reader> {
    raw: Raw<'reader, R>,
    pub dwarf: OnceCell<Dwarf<Relocator<'reader>>>,
}

impl<'reader, R> DwarfReader<'reader, R>
where
    R: ReadRef<'reader> + 'reader,
{
    ///
    ///
    ///
    pub fn new(raw: Raw<'reader, R>) -> Self {
        Self {
            raw,
            dwarf: OnceCell::new(),
        }
    }

    ///
    ///
    ///
    pub fn get(&'reader self) -> Result<&Dwarf<Relocator<'reader>>, Error> {
        self.dwarf.get().ok_or(()).or_else(|_| self.load())
    }
}

impl<'reader, R> DwarfReader<'reader, R>
where
    R: ReadRef<'reader> + 'reader,
{
    ///
    ///
    ///
    fn load(&'reader self) -> Result<&Dwarf<Relocator<'reader>>, Error> {
        // If the WASM debug info is in a split DWARF object (DWO), then load
        // the parent object first, so we can link them. The parent archive
        // contains references to the DWO object we resolve later in generating
        // the source map
        let parent = if let Some(parent) = &self.raw.dwo_parent {
            let load_parent_section =
                |id: gimli::SectionId| Self::load_file_section(id, parent, false);
            Some(gimli::Dwarf::load(load_parent_section)?)
        } else {
            None
        };
        let parent = parent.as_ref();

        // This is the target object binary we are generating the sourcemap for
        let load_section =
            |id: gimli::SectionId| Self::load_file_section(id, &self.raw.binary, parent.is_some());

        let mut dwarf = gimli::Dwarf::load(load_section)?;

        if parent.is_some() {
            if let Some(parent) = parent {
                dwarf.make_dwo(parent);
            } else {
                dwarf.file_type = gimli::DwarfFileType::Dwo;
            }
        }

        // Load optional supplemental file
        if let Some(sup) = &self.raw.sup_file {
            let load_sup_section = |id: gimli::SectionId| {
                // Note: we really only need the `.debug_str` section,
                // but for now we load them all.
                Self::load_file_section(id, sup, false)
            };
            dwarf.load_sup(load_sup_section)?;
        }

        dwarf.populate_abbreviations_cache(gimli::AbbreviationsCacheStrategy::All);

        Ok(self.dwarf.get_or_init(|| dwarf))
    }

    ///
    ///
    ///
    fn load_file_section(
        id: gimli::SectionId,
        object: &'reader File<'reader, R>,
        is_dwo: bool,
    ) -> Result<Relocator<'reader>, Error> {
        let mut relocations = RelocationMap::default();
        let name = if is_dwo {
            id.dwo_name()
        } else {
            Some(id.name())
        };

        let data = match name.and_then(|name| object.section_by_name(name)) {
            Some(ref section) => {
                // DWO sections never have relocations, so don't bother.
                if !is_dwo {
                    // Collect the relocations in this section and add to the relocation map
                    relocations.extend(Self::get_relocations(object, section)?);
                }
                section.uncompressed_data()?
            }
            // Use a non-zero capacity so that `ReaderOffsetId`s are unique.
            None => Cow::Owned(Vec::with_capacity(1)),
        };

        let reader = gimli::EndianReader::new(SectionReader { data }, LittleEndian);
        let offset = reader.offset_from(&reader);
        Ok(Relocate {
            relocations: Rc::new(relocations),
            offset,
            reader,
        })
    }

    ///
    ///
    ///
    fn get_relocations(
        object: &File<'reader, R>,
        section: &Section<'reader, 'reader, R>,
    ) -> Result<RelocationMap, Error> {
        let mut relocations: RelocationMap = RelocationMap::new();

        for (offset64, mut relocation) in section.relocations() {
            let offset = offset64 as usize;
            if offset as u64 != offset64 {
                continue;
            }

            match relocation.kind() {
                RelocationKind::Absolute => {
                    if let RelocationTarget::Symbol(symbol_idx) = relocation.target() {
                        match object.symbol_by_index(symbol_idx) {
                            Ok(symbol) => {
                                let addend =
                                    symbol.address().wrapping_add(relocation.addend() as u64);
                                relocation.set_addend(addend as i64);
                            }
                            Err(_) => {
                                let msg = format!(
                                    "Relocation with invalid symbol for section {} at offset 0x{:08x}",
                                    section.name().unwrap(),
                                    offset
                                );
                                return Err(msg.into());
                            }
                        }
                    }

                    if relocations.insert(offset, relocation).is_some() {
                        let msg = format!(
                            "Multiple relocations for section {} at offset 0x{:08x}",
                            section.name().unwrap(),
                            offset
                        );
                        return Err(msg.into());
                    }
                }
                _ => {
                    let msg = format!(
                        "Unsupported relocation for section {} at offset 0x{:08x}",
                        section.name().unwrap(),
                        offset
                    );
                    return Err(msg.into());
                }
            }
        }

        Ok(relocations)
    }
}

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
