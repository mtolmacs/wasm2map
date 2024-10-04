#![warn(missing_docs)]
#![warn(clippy::use_self)]
//! Having a sourcemap associated with your WASM file allows seeing the exact
//! filename, the line number and character position right in the browser or
//! supporting debugger. This can speed up tracing errors back to their source,
//! make sense of panic unwinds right in the browser and even simple console
//! messages are immediately identifiable without external post processing.
//!
//! It also offers an opportunity to debug the WASM binary, set breakpoints and
//! overall support the same developer experience JavaScript has in modern
//! browsers for ages.
//!
//! Inspirations:
//! * [wasm_sourcemap.py](https://github.com/emscripten-core/emscripten/blob/main/tools/wasm-sourcemap.py) by the Emscripten Team
//! * [WebAssembly Debugging](https://medium.com/oasislabs/webassembly-debugging-bec0aa93f8c6) by Will Scott and Oasis Labs

mod dwarf;
mod error;
#[cfg(feature = "loader")]
mod loader;
#[cfg(test)]
mod test;

use dwarf::DwarfReader;
pub use error::Error;
use error::InternalError;
use gimli::{self, Reader};
#[rustversion::before(1.65)]
use ilog::IntLog;
#[cfg(feature = "loader")]
pub use loader::WasmLoader;
use normalize_path::NormalizePath;
pub use object::ReadRef;
use object::{self, File, FileKind, Object, ObjectSection, SectionIndex};
use once_cell::unsync::OnceCell;
use sourcemap::SourceMapBuilder;
use std::{path::PathBuf, str};

type Entry = (u32, u32, u32, u32, Option<u32>, Option<u32>, bool);

///
pub struct Wasm<'wasm, R: ReadRef<'wasm>> {
    binary: File<'wasm, R>,
    dwo_parent: Option<File<'wasm, R>>,
    sup_file: Option<File<'wasm, R>>,
    offset: u32,
    dwarf: OnceCell<DwarfReader<'wasm, R>>,
}

impl<'wasm, R: ReadRef<'wasm>> Wasm<'wasm, R> {
    ///
    ///
    ///
    pub fn new(binary: R, dwo_parent: Option<R>, sup_file: Option<R>) -> Result<Self, Error> {
        let file = File::parse(binary)?;
        let offset = file
            .section_by_index(SectionIndex(10))?
            .file_range()
            .ok_or(InternalError::Generic(
                "The code section in the WASM file does not contain a size parameter".into(),
            ))?
            .0
            .try_into()
            .map_err(InternalError::from)?;

        Ok(Self {
            binary: match FileKind::parse(binary)? {
                FileKind::Wasm => Ok(file),
                _ => Err(InternalError::Generic(
                    "Object does not represent a WASM file".into(),
                )),
            }?,
            dwo_parent: if let Some(dwo_parent) = dwo_parent {
                let dwo_parent = match FileKind::parse(dwo_parent)? {
                    FileKind::Wasm => Ok(File::parse(dwo_parent)?),
                    _ => Err(InternalError::Generic(
                        "DWO parent file is not connected to a WASM file".into(),
                    )),
                }?;
                Some(dwo_parent)
            } else {
                None
            },
            sup_file: if let Some(sup_file) = sup_file {
                let sup_file = match FileKind::parse(sup_file)? {
                    FileKind::Wasm => Ok(File::parse(sup_file)?),
                    _ => Err(InternalError::Generic(
                        "Supplemental file is not connected to a WASM file".into(),
                    )),
                }?;
                Some(sup_file)
            } else {
                None
            },
            offset,
            dwarf: OnceCell::new(),
        })
    }

    ///
    ///
    ///
    pub fn build(&'wasm self, _bundle_sources: bool, _name: Option<&str>) -> Result<String, Error> {
        let mut entries: Vec<Entry> = Vec::new();
        let mut mapper = SourceMapBuilder::new(None);

        let dwarf = self
            .dwarf
            .get_or_init(|| {
                DwarfReader::new(
                    &self.binary,
                    self.dwo_parent.as_ref(),
                    self.sup_file.as_ref(),
                )
            })
            .get()?;

        let mut iter = dwarf.units();
        while let Some(header) = iter.next()? {
            let unit = match dwarf.unit(header) {
                Ok(unit) => unit,
                Err(_) => continue,
            };

            if let Some(program) = unit.line_program.clone() {
                let mut rows = program.clone().rows();
                while let Some((line_header, row)) = rows.next_row()? {
                    let line: u32 = match row.line() {
                        Some(line) => line.get().try_into().map_err(InternalError::from)?,
                        None => continue,
                    };
                    let column: u32 = match row.column() {
                        gimli::ColumnType::Column(column) => {
                            column.get().try_into().map_err(InternalError::from)?
                        }
                        gimli::ColumnType::LeftEdge => 0,
                    };
                    let mut address = row.address().try_into().map_err(InternalError::from)?;
                    address += self.offset;
                    let file = match row.file(line_header) {
                        Some(file) => {
                            let mut file_name = PathBuf::from(
                                dwarf
                                    .attr_string(&unit, file.path_name())?
                                    .to_string_lossy()?
                                    .as_ref(),
                            );

                            if let Some(directory_attr) = file.directory(program.header()) {
                                if let Ok(directory) = dwarf.attr_string(&unit, directory_attr) {
                                    if let Ok(directory) = directory.to_string_lossy() {
                                        file_name =
                                            PathBuf::from(directory.as_ref()).join(file_name);
                                    }
                                }
                            }
                            let sid = mapper.add_source(
                                file_name
                                    .normalize()
                                    .to_str()
                                    .ok_or(InternalError::Generic(
                                        "Error converting source file path to string".into(),
                                    ))?
                                    .replace('\\', "/")
                                    .as_str(),
                            );
                            Some(sid)
                        }
                        None => None,
                    };
                    let eos = row.end_sequence();

                    // TODO: Bundle sources?

                    if eos {
                        address -= 1;
                        let last = entries.last_mut().unwrap();
                        if last.1 == address {
                            last.6 = true;
                        }
                    }

                    entries.push((
                        0,
                        address,
                        line.saturating_sub(1),
                        column.saturating_sub(1),
                        file,
                        None, // TODO: Look up name
                        eos,
                    ));
                }
            }
        }

        //Self::remove_dead_entries(&mut entries);
        entries.sort_by(|left, right| left.1.cmp(&right.1));
        entries
            .into_iter()
            //.filter(|item| !item.6)
            .for_each(|(dst_line, dst_col, src_line, src_col, source, name, _)| {
                mapper.add_raw(dst_line, dst_col, src_line, src_col, source, name);
            });

        let mut buf: Vec<u8> = Vec::new();
        mapper.into_sourcemap().to_writer(&mut buf).unwrap();

        Ok(String::from_utf8(buf).unwrap())
    }

    ///
    ///
    fn _remove_dead_entries(entries: &mut Vec<Entry>) {
        let mut block_start = 0;
        let mut cur_entry = 0;
        while cur_entry < entries.len() {
            if !entries.get(cur_entry).unwrap().6 {
                cur_entry += 1;
            } else {
                let fn_start = entries.get(block_start).unwrap().1;
                let fn_ptr = entries.get(cur_entry).unwrap().1;
                let fn_size_length = (fn_ptr - fn_start + 1).ilog(128) + 1;
                let min_live_offset = 1 + fn_size_length;
                if fn_start < min_live_offset {
                    cur_entry += 1;
                    entries.as_mut_slice()[block_start..cur_entry]
                        .iter_mut()
                        .for_each(|e| e.6 = true);
                    cur_entry += 1;
                    continue;
                }
                cur_entry += 1;
                block_start = cur_entry;
            }
        }
    }
}

#[rustversion::before(1.65)]
//#[allow(unstable_name_collisions)]
trait PolyfillIlog {
    fn ilog(self, base: u32) -> u32;
}

#[rustversion::before(1.65)]
impl PolyfillIlog for u32 {
    fn ilog(self, base: u32) -> u32 {
        u32::try_from(self.log2() / base.log2()).expect(
            "Invariant of logarithm with arbitrary base from u32 cannot be converted to u32",
        )
    }
}
