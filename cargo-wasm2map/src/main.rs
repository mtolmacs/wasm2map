#![warn(missing_docs)]
//! This cargo command transforms the DWARF debug info in your WASM build to
//! standard browser-recognized Sourcemap format and appends the sourcemap
//! configuration to the WASM binary.
//!
//! # Usage
//! ```
//! cargo install cargo-wasm2map
//! cargo build --target wasm32-unknown-unknown
//! cargo wasm2map target/wasm32-unknown-unknown/myproject.wasm -p -base-url http://localhost:8080
//!
//! # <Load the index.html with your WASM in your browser...>
//! ```

use clap::{Args, Parser};
use std::{fs, path::PathBuf};
use wasm2map::WASM;

// Cargo commands receive the name of the subcommand as the main command
// so we need to consume the name of our executable in order to get to the
// actual params (i.e. cargo wasm2map ...)
#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum CargoCli {
    Wasm2map(WasmFile),
}

#[derive(Args)]
#[command(author, version, about, long_about = None)]
struct WasmFile {
    // The first argument is the WASM file path to process
    #[arg(help = "The path to the WASM file with debug info embedded (DWARF)")]
    path: PathBuf,

    #[arg(
        short,
        long,
        help = "Override default sourcemap file path (incl. filename)"
    )]
    map_path: Option<PathBuf>,

    #[arg(
        short,
        long,
        requires = "base_url",
        help = "Patch the WASM file with the sourcemap url section"
    )]
    patch: bool,

    #[arg(
        short,
        long,
        requires = "patch",
        help = "Base URL of the sourcefile where it can be fetched from"
    )]
    base_url: Option<String>,
}

fn main() -> Result<(), String> {
    // Parse the command parameters
    let CargoCli::Wasm2map(mut args) = CargoCli::parse();

    // Check if the WASM path points to a file
    if !args.path.is_file() {
        return Err(format!(
            "The WASM file path provided is not a file, {} was provided",
            args.path.display()
        ));
    }

    // Parse the --mapfile parameter or set a default
    // path based on the WASM file path and filename
    let map = if let Some(map) = args.map_path.take() {
        if !map.is_file() {
            return Err(format!(
                "The argument --mapfile must be a filepath, {} was provided",
                map.display()
            ));
        }

        map
    } else {
        // No --mapfile parameter, so by default take the
        // WASM file path and append ".map" to the path
        let mut map = args.path.clone();
        let mut filename = args.path.file_name().unwrap().to_owned();
        filename.push(".map");
        map.set_file_name(filename);

        map
    };

    // TODO(mtolamcs): Test the base url parameter to make sure its a valid
    // url and it also does not reference the map file

    // Load the WASM file to memory and parse the DWARF code section
    let mut wasm = WASM::load(&args.path).map_err(|err| err.to_string())?;

    // Generate the source map JSON for the loaded WASM
    let sourcemap = wasm.map_v3();

    // Dump JSON to the map file
    fs::write(&map, sourcemap).map_err(|err| err.to_string())?;

    // If patching is requested, then patch the WASM file at the parameter
    // with the provided source bap base url + the mapfile name
    if args.patch {
        let url = format!(
            "{}/{}",
            args.base_url.unwrap().as_str(),
            map.file_name().unwrap().to_str().unwrap()
        );
        wasm.patch(&url).map_err(|err| err.to_string())?;
    }

    Ok(())
}
