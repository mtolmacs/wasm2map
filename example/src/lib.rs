pub mod mem;

mod js {
    #[link(wasm_import_module = "console")]
    extern "C" {
        pub fn error(s: *const u8);
    }
}

pub fn error(s: &str) {
    unsafe {
        js::error(mem::into(s));
    }
}

#[no_mangle]
pub extern "C" fn start() {
    std::panic::set_hook(Box::new(|info: &std::panic::PanicInfo| {
        let loc_string;
        if let Some(location) = info.location() {
            loc_string = format!(
                "({}:{}:{})",
                std::path::Path::new(location.file())
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap(),
                location.line(),
                location.column()
            );
        } else {
            loc_string = "(<unknown>:<unknown>:<unknown>)".to_owned()
        }

        let error_message;
        if let Some(s) = info.payload().downcast_ref::<&str>() {
            error_message = format!("Panic occurred: {:?} at {}\n\n", s, loc_string);
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            error_message = format!("Panic occurred: {:?} at {}\n\n", s, loc_string);
        } else {
            error_message = format!("Unknown panic occurred at {}\n\n", loc_string);
        }

        error(&error_message);
    }));

    panic!("EXAMPLE PANIC")
}
