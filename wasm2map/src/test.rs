use std::{fs, ops::Deref, path::PathBuf};

use crate::{error::Error, json::encode, vlq, CodePoint, WASM};

// Consts needed to build golden versions of the binary WASM module section.
// See wasm2map::WASM::patch() doc-comment for details.
const WASM_CUSTOM_SECTION_ID: u8 = 0;
const WASM_SOURCEMAPPINGURL_SECTION_NAME: &[u8] = b"sourceMappingURL";

// TODO: Test sourcemap size load
// TODO: Test sourcemap generation

#[test]
fn can_create_sourcemap() {
    testutils::run_test(|out| {
        if let Ok(mapper) = WASM::load(&out) {
            let sourcemap = mapper.map_v3(false);

            assert!(sourcemap.starts_with(r#"{"version":3,"names":[],"sources":["#));
            assert!(sourcemap.ends_with(r#""}"#));
        } else {
            unreachable!()
        }
    });
}

#[test]
fn relative_paths_are_considered() {
    testutils::run_test(|out| {
        if let Ok(mapper) = WASM::load(&out) {
            let sourcemap = mapper.map_v3(false);

            // Any fixed relative path should have at least a `/` beforehand.
            #[cfg(target_os = "windows")]
            {
                // TODO(mtolmacs): The slashes on Windows need to have a
                // more robust matching method
                assert!(sourcemap.contains(r#"\library/core/src\any.rs"#));
                assert!(sourcemap.contains(r#"\library/core/src\panicking.rs"#));
            }
            #[cfg(not(target_os = "windows"))]
            {
                assert!(sourcemap.contains("/library/core/src/any.rs"));
                assert!(sourcemap.contains("/library/core/src/panicking.rs"));
            }
        } else {
            unreachable!()
        }
    });
}

#[test]
fn can_bundle_source() {
    testutils::run_test(|out| {
        if let Ok(mapper) = WASM::load(&out) {
            let sourcemap = mapper.map_v3(true);
            assert!(sourcemap.contains("fn main() {}"));
        } else {
            unreachable!()
        }
    });
}

#[test]
fn can_add_and_update_sourcemap() {
    testutils::run_test(|out| {
        // Set up the test byte data
        const URL: &'static str = "http://localhost:8080";
        let content = [
            &[WASM_SOURCEMAPPINGURL_SECTION_NAME.len() as u8],
            WASM_SOURCEMAPPINGURL_SECTION_NAME,
            &[URL.len() as u8],
            URL.as_bytes(),
        ]
        .concat();
        let section = [
            &[WASM_CUSTOM_SECTION_ID] as &[u8],
            &[content.len() as u8],
            content.as_ref(),
        ]
        .concat();
        const URL2: &'static str = "http://127.0.0.1:8080";
        let content2 = [
            &[WASM_SOURCEMAPPINGURL_SECTION_NAME.len() as u8],
            WASM_SOURCEMAPPINGURL_SECTION_NAME,
            &[URL2.len() as u8] as &[u8],
            URL2.as_bytes(),
        ]
        .concat();
        let section2 = [
            &[WASM_CUSTOM_SECTION_ID] as &[u8],
            &[content2.len() as u8],
            content2.as_ref(),
        ]
        .concat();

        let mapper = WASM::load(&out);
        if let Ok(mut mapper) = mapper {
            // Patch the WASM with sourceMappingURL and check if it is applied
            // correctly
            if let Err(error) = mapper.patch(URL) {
                panic!("Failed to patch the WASM file the first time: {}", error);
            }
            {
                let test = testutils::peek_wasm_file_end(out.clone(), section.len());
                assert_eq!(test, section);
            }

            // Update it and check if it's still valid and not duplicated
            if let Err(error) = mapper.patch(URL2) {
                panic!("Failed to patch the WASM file the first time: {}", error);
            }
            {
                let test =
                    testutils::peek_wasm_file_end(out.clone(), section.len() + section2.len());

                // Test if the patch just keeps adding patches or properly
                // update the old one
                assert_ne!(
                    test,
                    [section.as_ref() as &[u8], section2.as_ref()].concat()
                );

                // Test if the only sourceMappingURL is the last one we set
                assert_eq!(test[test.len() - section2.len()..], section2);
            }

            // Attempt to patch with the last one for sanity check
            if let Err(error) = mapper.patch(URL2) {
                panic!("Failed to patch the WASM file the first time: {}", error);
            }
            {
                // Test if WASM binary is at least structurally valid
                let raw = fs::read(&out).expect("Cannot open the WASM file");
                let obj = object::File::parse(raw.deref());
                assert!(obj.is_ok());
            }
        } else {
            panic!("Error loading WASM: {}", mapper.err().unwrap());
        }

        let mapper = WASM::load(&out);
        if let Ok(mut mapper) = mapper {
            // Attempt to patch with the last one for sanity check
            if let Err(error) = mapper.patch(URL2) {
                panic!("Failed to patch the WASM file the first time: {}", error);
            }
            {
                // Test if WASM binary is at least structurally valid
                let raw = fs::read(&out).expect("Cannot open the WASM file");
                let obj = object::File::parse(raw.deref());
                assert!(obj.is_ok());
            }
        } else {
            panic!("Error loading WASM: {}", mapper.err().unwrap());
        }
    })
}

#[test]
fn test_path_handles_nonexistent_wasm() {
    testutils::run_test(|out| {
        let mapper = WASM::load(&out);
        if let Ok(mut mapper) = mapper {
            // Delete the WASM file to trigger error
            fs::remove_file(&out).ok();

            // Attempt to patch with the last one for sanity check
            let result = mapper.patch("http://127.0.0.1:8080");

            assert!(result.is_err())
        } else {
            panic!("Error loading WASM: {}", mapper.err().unwrap());
        }
    });
}

#[test]
fn test_error_types() {
    fn errors() -> Result<(), Box<dyn std::error::Error>> {
        let _error: crate::Error =
            std::io::Error::new(std::io::ErrorKind::AddrInUse, "This is a test").into();

        let dumbarray = Vec::<u8>::new();
        let _error: crate::Error = match object::File::parse(dumbarray.as_slice()) {
            Ok(_) => unreachable!(),
            Err(err) => err.into(),
        };

        let _error: crate::Error = gimli::Error::Io.into();

        let _error: crate::Error = "This is a test".into();

        let _error: crate::Error = "This is a test".to_owned().into();

        let num: Result<i32, std::num::TryFromIntError> = u32::MAX.try_into();
        let _error: crate::Error = match num {
            Ok(_) => unreachable!(),
            Err(err) => err.into(),
        };

        Err(Box::from(_error))
    }

    let errors = errors();
    assert!(errors.is_err());

    let error: crate::Error = "This is a test".into();
    assert_eq!(format!("{}", error), "This is a test");
}

#[test]
fn test_numeric_encode_to_byte_sequence() {
    assert_eq!(vlq::encode_uint_var(432), vec![176, 3])
}

#[test]
fn test_derived_macros_present() {
    testutils::run_test(|out| {
        let codepoint = CodePoint {
            path: PathBuf::new(),
            address: 0,
            line: 0,
            column: 0,
        };
        assert!(format!("{:#?}", codepoint).len() > 0);
        let wasm =
            WASM::load(out).expect("Loading WASM file is unsuccessful in derived macros test");
        assert!(format!("{:#?}", wasm).len() > 0);
        let error = Error::from("");
        assert!(format!("{:#?}", error).len() > 0);
    })
}

#[test]
fn test_json_encode() {
    let buf = [0; 32]
        .iter()
        .enumerate()
        .map(|(count, _)| u8::try_from(count).expect("Data buffer is longer than 32"))
        .collect::<Vec<u8>>();
    assert_eq!(
        encode(std::str::from_utf8(buf.as_slice()).expect("Wrong test buffer data")),
        r#"\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\b\t\n\u000b\f\r\u000e\u000f\u0010\u0011\u0012\u0013\u0014\u0015\u0016\u0017\u0018\u0019\u001a\u001b\u001c\u001d\u001e\u001f"#
    );
    let buf2 = &[36, 35, 34, 92, 93, 94];
    assert_eq!(
        encode(std::str::from_utf8(buf2.as_slice()).expect("Wrong second test buffer data")),
        r#"$#\"\\"#
    );
}

mod testutils {
    use std::{
        fs, panic,
        path::PathBuf,
        process::{Command, Stdio},
    };

    // Get the target dir for the project or workspace directly from cargo
    // so we can create the temporary WASM file somewhere reliable
    pub fn get_target_dir() -> PathBuf {
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
        let mut file = get_target_dir();
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
        let mut out = get_target_dir();
        out.push("target");
        out.push(format!("test{}.wasm", get_thread_id()));

        build_with_rustc("fn main() {}", out.display().to_string().as_str());

        out.to_string_lossy().to_string()
    }

    // Remove the test WASM at the end of each test case
    pub fn teardown() {
        let mut target = get_target_dir();
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
    pub fn run_test<T>(test: T) -> ()
    where
        T: FnOnce(String) -> () + panic::UnwindSafe,
    {
        let out = setup();
        let result = panic::catch_unwind(|| test(out));
        teardown();
        assert!(result.is_ok())
    }
}
