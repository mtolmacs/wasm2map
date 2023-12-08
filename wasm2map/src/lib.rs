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
// #[cfg(test)]
// mod test;

use dwarf::DwarfReader;
use error::Error;
use gimli::{self, Reader};
#[cfg(feature = "loader")]
pub use loader::WasmLoader;
pub use object::ReadRef;
use object::{self, File};
use sourcemap::SourceMapBuilder;
use std::{borrow::Cow, cell::OnceCell, str};

///
pub struct Wasm<'wasm, R: ReadRef<'wasm>> {
    binary: File<'wasm, R>,
    dwo_parent: Option<File<'wasm, R>>,
    sup_file: Option<File<'wasm, R>>,
    dwarf: OnceCell<DwarfReader<'wasm, R>>,
}

impl<'wasm, R: ReadRef<'wasm>> Wasm<'wasm, R> {
    ///
    ///
    ///
    pub fn new(binary: R, dwo_parent: Option<R>, sup_file: Option<R>) -> Result<Self, Error> {
        Ok(Self {
            binary: match File::parse(binary)? {
                file @ File::Wasm(_) => Ok(file),
                _ => Err(Error::from("Object does not represent a WASM file")),
            }?,
            dwo_parent: if let Some(dwo_parent) = dwo_parent {
                let dwo_parent = match File::parse(dwo_parent)? {
                    file @ File::Wasm(_) => Ok(file),
                    _ => Err(Error::from(
                        "DWO parent object does not represent a WASM file",
                    )),
                }?;
                Some(dwo_parent)
            } else {
                None
            },
            sup_file: if let Some(sup_file) = sup_file {
                let sup_file = match File::parse(sup_file)? {
                    file @ File::Wasm(_) => Ok(file),
                    _ => Err(Error::from(
                        "Supplemental file does not represent a WASM file",
                    )),
                }?;
                Some(sup_file)
            } else {
                None
            },
            dwarf: OnceCell::new(),
        })
    }

    ///
    ///
    ///
    pub fn build(&'wasm self, bundle_sources: bool, name: Option<&str>) -> Result<String, Error> {
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

        let mut mapper = SourceMapBuilder::new(None);
        let mut iter = dwarf.units();
        while let Some(header) = iter.next()? {
            let unit = match dwarf.unit(header) {
                Ok(unit) => unit,
                Err(_) => continue,
            };
            if let Some(program) = unit.line_program.clone() {
                //let header = program.header();
                //let base = if header.version() >= 5 { 0 } else { 1 };
                //header.directory(directory)
                let mut rows = program.clone().rows();
                while let Some((line_header, row)) = rows.next_row()? {
                    let line = match row.line() {
                        Some(line) => line.get(),
                        None => 0,
                    };
                    let column = match row.column() {
                        gimli::ColumnType::Column(column) => column.get(),
                        gimli::ColumnType::LeftEdge => 0,
                    };
                    let file = match row.file(line_header) {
                        Some(file) => {
                            let mut file_name = dwarf
                                .attr_string(&unit, file.path_name())?
                                .to_string_lossy()?
                                .to_string();
                            if let Some(directory_attr) = file.directory(program.header()) {
                                if let Ok(directory) = dwarf.attr_string(&unit, directory_attr) {
                                    if let Ok(directory) = directory.to_string_lossy() {
                                        let mut directory = directory.to_string();
                                        directory.push('/');
                                        file_name.insert_str(0, &directory);
                                    }
                                }
                            }
                            let sid = mapper.add_source(file_name.as_ref());
                            Some(sid)
                        }
                        None => None,
                    };

                    // TODO: Bundle sources?

                    mapper.add_raw(
                        1,
                        row.address().try_into()?,
                        line.try_into()?,
                        column.try_into()?,
                        file,
                        None, // TODO: Look up name
                    );

                    //if row.end_sequence() {}
                }
            }
        }

        let mut buf: Vec<u8> = Vec::new();
        mapper.into_sourcemap().to_writer(&mut buf).unwrap();

        Ok(String::from_utf8(buf).unwrap())
    }
}
