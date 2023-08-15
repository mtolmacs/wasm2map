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

mod error;
mod json;
#[cfg(test)]
mod test;
mod vlq;

use error::Error;
use object::{Object, ObjectSection};
use std::{
    borrow::Cow,
    collections::BTreeMap,
    fs,
    io::{self, Seek, Write},
    ops::Deref,
    path::{Path, PathBuf},
    str,
};

const DWARF_CODE_SECTION_ID: usize = 10;

/// Represents a code unit which can be translated to a sourcemap code point
#[derive(Debug)]
pub struct CodePoint {
    path: PathBuf,
    address: i64,
    line: i64,
    column: i64,
}

/// The actual DWARF to Sourcemap mapper
///
/// # Usage
///
/// ```rust
/// use wasm2map::WASM;
///
/// let mapper = WASM::load("/path/to/the/file.wasm");
/// if let Ok(mut mapper) = mapper {
///     let sourcemap = mapper.map_v3(false);
///     mapper.patch("http://localhost:8080").expect("Failed to patch");
/// }
/// ```
#[derive(Debug)]
pub struct WASM {
    path: PathBuf,
    points: BTreeMap<i64, CodePoint>,
    sourcemap_size: Option<u64>,
}

struct Generated {
    mappings: Vec<String>,
    sources: Vec<String>,
    contents: Option<Vec<Cow<'static, str>>>,
}

impl WASM {
    /// Loads the WASM file under 'path' into memory and parses the DWARF info
    /// If the WASM or the DWARF info in it is malformed (or non-existent)
    /// it returns with the appropriate error result.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref().to_owned();

        #[cfg(feature = "memmap2")]
        let raw = {
            // Load the WASM file into memory via mmap to speed things up
            // with large WASM files
            let file = fs::File::open(&path)?;
            unsafe { memmap2::Mmap::map(&file) }?
        };
        #[cfg(not(feature = "memmap2"))]
        let raw = {
            // Load the WASM file via the standard library, which can be slower
            // for larger WASM files, but some platforms might not be supported
            // by memmap2
            fs::read(&path)?
        };

        // Parse the modules and sections from the WASM
        let object = object::File::parse(raw.deref())?;

        // Load the sourcemap custom section (if any) and calculate the total
        // size of the whole custom module (that is, the sourceMappingURL module)
        let sourcemap_size = match object.section_by_name("sourceMappingURL") {
            Some(section) => {
                // This is the '0' section type
                const CUSTOM_SEGMENT_ID_SIZE: u64 = 1;
                // The size of the length b"sourceMappingURL" (which is always
                // 1 byte, so the size of u8) + the length of the
                // b"sourceMappingURL" byte array
                const SEGMENT_NAME_SIZE: u64 =
                    std::mem::size_of::<u8>() as u64 + b"sourceMappingURL".len() as u64;
                let section_size_length = vlq::encode_uint_var(section.size() as u32).len() as u64;
                let section_size = CUSTOM_SEGMENT_ID_SIZE
                    + SEGMENT_NAME_SIZE
                    + section_size_length
                    + section.size();
                Some(section_size)
            }
            None => None,
        };

        // Load the code section to get its offset
        let offset: i64 = {
            let (code_section_offset, _) = object
                .section_by_index(object::SectionIndex(DWARF_CODE_SECTION_ID))?
                .file_range()
                .ok_or("Missing code section in WASM")?;
            code_section_offset.try_into()?
        };

        // Load all of the DWARF sections
        let section =
            gimli::Dwarf::load(|id: gimli::SectionId| -> Result<Cow<[u8]>, gimli::Error> {
                match object.section_by_name(id.name()) {
                    Some(ref section) => Ok(section
                        .uncompressed_data()
                        .unwrap_or(Cow::Borrowed(&[][..]))),
                    None => Ok(Cow::Borrowed(&[][..])),
                }
            })?;

        // Borrow a `Cow<[u8]>` to create an `EndianSlice`.
        let borrow_section: &dyn for<'a> Fn(
            &'a Cow<[u8]>,
        )
            -> gimli::EndianSlice<'a, gimli::RunTimeEndian> =
            &|section| gimli::EndianSlice::new(section, gimli::RunTimeEndian::Little);

        // Create `EndianSlice`s for all of the sections.
        let dwarf = section.borrow(&borrow_section);

        // Collect the debug data and enforce that they are sorted by address
        // which BTreeMap guarantees
        let mut points: BTreeMap<i64, CodePoint> = BTreeMap::new();

        let mut iter = dwarf.units();
        while let Some(header) = iter.next()? {
            let unit = dwarf.unit(header)?;

            // let mut depth = 0;
            // let mut entries = unit.entries();
            // while let Some((delta_depth, entry)) = entries.next_dfs()? {
            //     depth += delta_depth;
            //     println!("<{}><{:x}> {}", depth, entry.offset().0, entry.tag());

            //     // Iterate over the attributes in the DIE.
            //     let mut attrs = entry.attrs();
            //     while let Some(attr) = attrs.next()? {
            //         match attr.value() {
            //             gimli::AttributeValue::DebugStrRef(offset) => {
            //                 if let Ok(slice) = dwarf.debug_str.get_str(offset) {
            //                     println!("   {}: {:?}", attr.name(), slice.to_string_lossy());
            //                 } else {
            //                     println!("   {}: {:?}", attr.name(), attr.value());
            //                 }
            //             }
            //             _ => println!("   {}: {:?}", attr.name(), attr.value()),
            //         }
            //     }
            // }

            let mut depth = 0;
            let mut entries = unit.entries();
            while let Some((delta_depth, entry)) = entries.next_dfs()? {
                depth += delta_depth;

                let tag_string = entry.tag().to_string();
                if tag_string == "DW_TAG_subprogram" || tag_string == "DW_TAG_inline_subroutine" {
                    //println!("<{}><{:x}> {}", depth, entry.offset().0, entry.tag());
                    // Iterate over the attributes in the DIE.
                    let mut attrs = entry.attrs();
                    while let Some(attr) = attrs.next()? {
                        let val = attr.name().to_string();
                        if val == "DW_AT_linkage_name"
                            || val == "DW_AT_decl_line"
                            || val == "DW_AT_decl_file"
                            || val == "DW_AT_name"
                        {
                            if let gimli::AttributeValue::DebugStrRef(offset) = attr.value() {
                                if let Ok(slice) = dwarf.debug_str.get_str(offset) {
                                    let val = slice.to_string_lossy().to_string();
                                    //println!("   {}: {:?}", attr.name(), demangle(val.as_str()));
                                }
                            } else {
                                //println!("   {}: {:?}", attr.name(), attr.value());
                            }
                        }
                    }
                    //println!("------------")
                }
            }

            // Get the line program for the compilation unit.
            if let Some(program) = unit.line_program.clone() {
                // Iterate over the line program rows for the unit.
                let mut rows = program.rows();
                while let Some((header, row)) = rows.next_row()? {
                    // We will collect the embdedded path from the DWARF loc metadata
                    let mut path = PathBuf::new();

                    if let Some(file) = row.file(header) {
                        if let Some(dir) = file.directory(header) {
                            let dir: PathBuf = dwarf
                                .attr_string(&unit, dir)?
                                .to_string_lossy()
                                .as_ref()
                                .into();

                            // Relative directories are relative to the compilation unit directory.
                            if dir.as_path().is_relative() {
                                if let Some(dir) = unit.comp_dir {
                                    path.push(dir.to_string_lossy().as_ref())
                                }
                            }

                            path.push(dir);
                        }

                        let relative: String = dwarf
                            .attr_string(&unit, file.path_name())?
                            .to_string_lossy()
                            .into();
                        path.push(relative);
                        path = match path.to_str() {
                            Some(t) => t.replace('\\', "/").into(),
                            None => path,
                        };
                    }

                    // The address of the instruction in the code section
                    let address: i64 = {
                        let mut addr: i64 = row.address().try_into()?;
                        if row.end_sequence() {
                            addr -= 1;
                        }
                        addr + offset
                    };

                    // Determine line/column. DWARF line/column is never 0
                    let line = {
                        let line = match row.line() {
                            Some(line) => line.get(),

                            // No line information means this code block does not belong to
                            // a source code block (generated by the compiler for whatever
                            // reason)
                            None => 0,
                        };
                        line.try_into()?
                    };

                    let column: i64 = {
                        let col = match row.column() {
                            gimli::ColumnType::LeftEdge => 1,
                            gimli::ColumnType::Column(column) => column.get(),
                        };
                        col.try_into()?
                    };

                    let point = CodePoint {
                        path,
                        address,
                        line,
                        column,
                    };

                    points.insert(point.address, point);
                }
            }
        }

        Ok(Self {
            path,
            points,
            sourcemap_size,
        })
    }

    /// Generate the sourcemap v3 JSON from the parsed WASM DWARF data.
    ///
    /// The `bundle` parameter, when set to true, bundles the source code
    /// of your project in the source map, so you can jump to the source
    /// code from the console, not just the raw WASM bytecode.
    ///
    /// Note: The mapper is currently not able to package the source code
    /// of crate dependencies, nor the rust library sources.
    ///
    /// # Example output
    ///
    /// ```json
    /// {
    ///     "version": 3,
    ///     "names": [],
    ///     "sources": [
    ///         "file/path/name.rs",
    ///         "another/file/path.rs"
    ///         ...
    ///     ],
    ///     "sourcesContent": [
    ///         null,
    ///         null,
    ///         null,
    ///         "fn main() {}",
    ///         null,
    ///         ...
    ///     ],
    ///     "mappings": {
    ///         "yjBAiIA,qCAIiB,QAMhB,...,oBAAA"
    ///     }
    /// }
    /// ```
    pub fn map_v3(&self, bundle: bool) -> String {
        let mut sourcemap = String::with_capacity(self.points.len() * 4 + 100);
        let Generated {
            mappings,
            sources,
            contents,
        } = self.generate(bundle);

        sourcemap.push('{');
        sourcemap.push_str(r#""version":3,"#);
        if let Some(os_file_name) = self.path.file_name() {
            if let Some(file_name) = os_file_name.to_str() {
                sourcemap.push_str(format!(r#""file":"{}","#, file_name).as_str());
            }
        }
        sourcemap.push_str(r#""sourceRoot":"","#);
        sourcemap.push_str(r#""names":[],"#);
        let s: Vec<String> = sources
            .into_iter()
            .map(|source| {
                if let Some(pos) = source.find(':') {
                    source[pos + 1..].to_string()
                } else {
                    source
                }
            })
            //.map(|source| source.rsplit('/').next().expect("NO FILENAME").to_string())
            .collect();
        sourcemap.push_str(format!(r#""sources":["{}"],"#, s.join(r#"",""#)).as_str());

        if let Some(contents) = contents {
            debug_assert!(bundle);
            sourcemap.push_str(format!(r#""sourcesContent":[{}],"#, contents.join(",")).as_str());
        } else {
            sourcemap.push_str(r#""sourcesContent":null,"#);
        }

        sourcemap.push_str(format!(r#""mappings":"{}""#, mappings.join(",")).as_str());
        sourcemap.push('}');

        sourcemap
    }

    #[allow(rustdoc::invalid_html_tags)]
    /// Patch the loaded WASM file to reference the sourcemap and ask the
    /// browser or debugger to load it for us when referencing the code
    ///
    /// # Limitations
    /// This can only work if the sourceMappingURL custom section is the last
    /// section of the WASM.
    ///
    /// # How does this work?
    ///
    /// The WebAssembly specification contains a "custom" section definition
    /// which is used to encode the sourcemap url in the WASM binary.
    ///
    /// The structure of the custom module is as follows (without ):
    /// (
    ///     0 <section_length> (
    ///         <name_length> <name>
    ///         <urllen> <url>
    ///     )
    /// )
    ///
    /// This structure is VLQ encoded without the parentheses and spaces into
    /// a byte array and appended to the end of the WASM binary.
    ///
    /// More details in the [WebAssembly Module Specification](https://webassembly.github.io/spec/core/binary/modules.html)
    pub fn patch(&mut self, url: &str) -> Result<(), Error> {
        // Open WASM binary for writing
        let mut wasm = fs::OpenOptions::new()
            .write(true)
            .open(&self.path)
            .map_err(|err| {
                format!(
                    "Failed to open WASM file to append sourcemap section: {}",
                    err
                )
            })?;

        // Grab the actual size (byte count) of the WASM binary
        let size = wasm.seek(io::SeekFrom::End(0))?;

        // Determine the file cusrsor position without the custom section (if any)
        // by subtracting the size of the sourceMappingURL section from the
        // byte size of the WASM binary
        let pos = self
            .sourcemap_size
            .map(|length| size - length)
            .unwrap_or(size);

        // Truncate the WASM binary and position the file cursor to the new end
        // (if there was a sourcemap added), no-op otherwise
        wasm.set_len(pos)?;
        wasm.seek(io::SeekFrom::End(0))?;

        // Generate the souceMappingURL custom
        // section (see above for info on structure)
        const WASM_CUSTOM_SECTION_ID: u32 = 0;
        let section_name = "sourceMappingURL";
        let section_content = [
            &vlq::encode_uint_var(section_name.len() as u32)[..],
            section_name.as_bytes(),
            &vlq::encode_uint_var(url.len() as u32)[..],
            url.as_bytes(),
        ]
        .concat();
        let section = [
            &vlq::encode_uint_var(WASM_CUSTOM_SECTION_ID)[..],
            &vlq::encode_uint_var(section_content.len() as u32)[..],
            section_content.as_ref(),
        ]
        .concat();

        // Write out the custom section
        wasm.write_all(&section)
            .map_err(|err| format!("Failed to write sourcemap section to WASM file: {}", err))?;

        let _s = wasm.seek(io::SeekFrom::End(0));

        // Set the sourcemap data after writing it out
        self.sourcemap_size = Some(section.len() as u64);

        Ok(())
    }

    // Generate the sourcemap mappings and source ids.
    //
    // The sourcemap 3 format tries to save on file size by using offsets
    // wherever possible. So we need to encode the source file data and
    // line, column data for each WASM code segment address in the expected
    // order, so offsets make sense when resolved by the browser (or debugger)
    fn generate<'a>(&'a self, bundle: bool) -> Generated {
        // We collect all referenced source code files in a table and use the
        // source id (which is the value param of this HashMap) as the basis for
        // the offset when encoding position (i.e. last source id - this source id),
        // which require preserving the order of inserts!
        let mut sources: Vec<&'a Path> = Vec::new();
        //let mut sources: BTreeMap<&'a Path, i64> = BTreeMap::new();
        //let mut sources: HashMap<&'a Path, i64> = HashMap::new();

        // This is the WASM address -> file:line:col mapping table in the
        // required format, which is basically offsets written after each other
        // in the specified order (address, source id, line, finally col)
        let mut mappings: Vec<String> = Vec::new();

        // These variables track the last of the four pieces of data so we can
        // subtract from them to get an offset and then update them to the latest
        let mut last_address: i64 = 0;
        let mut last_source_id: i64 = 0;
        let mut last_line: i64 = 1;
        let mut last_column: i64 = 1;

        for line in self.points.values() {
            // Line 0 means that this is an intermediate code block and does not
            // refer to a code block in the source files. We need to skip these
            // in order to generate the proper offset encoding
            if line.line == 0 {
                continue;
            }

            // We either get the id of a source file if already in the table
            // or we get the max(id) + 1 as the new id for a previously unseen
            // source file, which we promptly insert into the source table
            let source_id: i64 =
                if let Some(id) = sources.iter().position(|&val| val == line.path.as_path()) {
                    id as i64
                } else {
                    let id = sources.len() as i64;
                    sources.push(&line.path);
                    id
                };

            // Calculate the offsets (see above)
            let address_delta = line.address - last_address;
            let source_id_delta = source_id - last_source_id;
            let line_delta = line.line - last_line;
            let column_delta = line.column - last_column;

            // Store the mapping offsets in the specific format
            // (see above) in the mapping table
            let mapping = format!(
                "{}{}{}{}",
                vlq::encode(address_delta).as_str(),
                vlq::encode(source_id_delta).as_str(),
                vlq::encode(line_delta).as_str(),
                vlq::encode(column_delta).as_str()
            );
            mappings.push(mapping);

            // Update the tracking variables to the freshly calculated values
            // to use them in the next iteration (see above)
            last_address = line.address;
            last_source_id = source_id;
            last_line = line.line;
            last_column = line.column;
        }

        // We only need the file paths from the sources table in the order
        // they were encoded, turned to strings
        let sources = sources
            .iter()
            .filter_map(|p| Some(p.as_os_str().to_str()?.to_owned()))
            .collect::<Vec<_>>();

        let contents = bundle.then(|| {
            sources
                .iter()
                .map(Path::new)
                .map(|path| {
                    fs::read_to_string(path)
                        .map(|content| Cow::Owned(format!(r#""{}""#, json::encode(&content))))
                        .unwrap_or(Cow::Borrowed("null"))
                })
                .collect()
        });

        Generated {
            mappings,
            sources,
            contents,
        }
    }
}
