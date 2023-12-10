// TODO: Test relocation
// TODO: Test DWO?

use sourcemap::SourceMap;

use crate::{Wasm, WasmLoader};
use std::{fs, panic, path::Path};

/// Tests the format of the sourcemap, makes sure the JSON is valid and
/// the required keys are present, with the right type of values.
#[test]
fn can_create_valid_sourcemap_format() {
    panic::catch_unwind(|| {
        let path = testutils::get_workspace_dir().join("wasm2map/test/assets/golden.wasm");
        let loader = WasmLoader::from_path(path).expect("Could not load WASM file");
        let wasm = Wasm::new(&loader, None, None).expect("Could not load WASM sections");
        if let Ok(sourcemap) = wasm.build(false, None) {
            let parsed = serde_json::from_str::<serde_json::Value>(sourcemap.as_str())
                .expect("Sourcemap is not a valid JSON file");
            let json = parsed.as_object().expect("Sourcemap is not a JSON object");

            let version = json
                .get("version")
                .expect("Sourcemap JSON object has no requied version key")
                .as_i64()
                .expect("Sourcemap JSON version value is not an integer");
            assert!(version == 3);

            let names = json
                .get("names")
                .expect("Sourcemap JSON has no names key")
                .as_array()
                .expect("Sourcemap JSON key names is not an array");
            assert!(names.is_empty());

            let sources = json
                .get("sources")
                .expect("Sourcemap JSON object has no sources key")
                .as_array()
                .expect("Sourcemap JSON sources value is not an array");
            assert!(!sources.is_empty());
            sources.iter().for_each(|value| {
                let path = Path::new(
                    value
                        .as_str()
                        .expect("Sourcemap JSON sources item is not a string"),
                );
                assert!(path.extension().is_some());
            });

            let mappings = json
                .get("mappings")
                .expect("Sourcemap JSON object has no mappings key")
                .as_str()
                .expect("Sourcemap JSON key mappings is not a string");
            assert!(!mappings.is_empty());
        } else {
            unreachable!("Could not load WASM binary")
        }
    })
    .expect("Cannot create a valid sourcemap format");
}

/// Check the address resolution in the generated sourcemap against a known good
/// example
#[test]
fn position_retrieval_works() {
    let golden = {
        let path = testutils::get_workspace_dir().join("wasm2map/test/assets/golden.wasm.map");
        let bytes = std::fs::read(path).expect("Could not load golden sourcemap");
        SourceMap::from_slice(&bytes).expect("Malformed golden sourcemap file")
    };

    let path = testutils::get_workspace_dir().join("wasm2map/test/assets/golden.wasm");
    let loader = WasmLoader::from_path(path).expect("Could not load WASM file");
    let wasm = Wasm::new(&loader, None, None).expect("Could not load WASM sections");
    let sourcemap = SourceMap::from_slice(
        wasm.build(false, None)
            .expect("Failed to build sourcemap for golden WASM")
            .as_bytes(),
    )
    .expect("Generated sourcemap is not valid");

    let mut entry: u32 = 0;
    golden.tokens().for_each(|golden_token| {
        entry += 1;
        let col = golden_token.get_dst_col();
        let line = golden_token.get_dst_line();
        let token = sourcemap.lookup_token(line, col).unwrap_or_else(|| {
            panic!(
                "Position {}:{} from golden.wasm.map is not present in the generated sourcemap at position {}",
                line, col, entry
            )
        });
        let left = golden_token.to_string();
        let right = token.to_string();

        assert!(
            left.as_str().eq(right.as_str()),
            "[{}] {} <=> {}",
            col,
            left,
            right
        );
    });
}

mod testutils {
    use std::{
        fs, panic,
        path::PathBuf,
        process::{Command, Stdio},
    };

    // Get the target dir for the project or workspace directly from cargo
    // so we can create the temporary WASM file somewhere reliable
    pub fn get_workspace_dir() -> PathBuf {
        let mut out = PathBuf::new();
        let raw = Command::new("cargo")
            .args(["locate-project", "--workspace"])
            .output()
            .expect("Failed to locate cargo project")
            .stdout;
        let locate_project = &String::from_utf8_lossy(&raw);
        out.push(&locate_project[9..locate_project.len() - 14]);
        out
    }

    // Invoke the rustc command to compile a simple WASM binary with DWARF info
    // so we can run our tests on it
    //
    // # Concept
    //
    // rustc gets the source code (the source param) from stdin and writes out
    // the created WASM binary to the project / workspace target dir.
    //
    // NOTE: We also force the WASM32 target obviously, so the tests need that toolchain
    pub fn build_with_rustc(source: &'_ str, output: &'_ str) {
        let mut file = get_workspace_dir();
        file.push("target");
        file.push(format!("test{}.rs", get_thread_id()));
        std::fs::write(&file, source).unwrap();

        let mut rustc = Command::new("rustc")
            .args(["--target", "wasm32-unknown-unknown", "-g", "-o", output])
            .arg(file)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Test WASM compile unsuccessful");
        rustc
            .wait()
            .expect("Could not compile test WASM successfully");
    }

    // Builds a test WASM file via rustc in the target directory for the tests
    // to manipulate
    pub fn setup() -> String {
        let mut out = get_workspace_dir();
        out.push("target");
        out.push(format!("test{}.wasm", get_thread_id()));

        build_with_rustc("fn main() {}", out.display().to_string().as_str());

        out.to_string_lossy().to_string()
    }

    // Remove the test WASM at the end of each test case
    pub fn teardown() {
        let mut target = get_workspace_dir();
        target.push("target");

        let mut wasm = target.clone();
        wasm.push(format!("test{}.wasm", get_thread_id()));
        fs::remove_file(wasm.as_path()).ok();

        let mut input = target.clone();
        input.push(format!("test{}.rs", get_thread_id()));
        fs::remove_file(input.as_path()).ok();
    }

    pub fn get_thread_id() -> u64 {
        // TODO(mtolmacs): There's an easier way on nightly, https://github.com/rust-lang/rust/issues/67939
        let str = format!("{:#?}", std::thread::current().id());
        let num = &str.as_str()[14..str.len() - 3];

        str::parse::<u64>(num).expect("ThreadId debug format changed")
    }

    // Loads 'loopback' bytes from the end of the WASM binary specified by the 'path'
    // parameter, which we can use to match against expected binary patters
    pub fn peek_wasm_file_end(path: String, lookback: usize) -> Vec<u8> {
        let binary = fs::read(path).expect("Can't open the test WASM file for reading");
        binary[binary.len() - lookback..].to_owned()
    }

    // Run a test with setup and teardown for the test case
    pub fn run_test<T>(test: T)
    where
        T: FnOnce(String) + panic::UnwindSafe,
    {
        let out = setup();
        let result = panic::catch_unwind(|| test(out));
        teardown();
        assert!(result.is_ok())
    }
}
