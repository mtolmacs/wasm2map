#[no_mangle]
extern "C" fn alloc(size: usize) -> *mut u8 {
    unsafe { std::alloc::alloc(std::alloc::Layout::from_size_align(size, 1).unwrap()) }
}

/// Frees len bytes of memory at ptr on the WASM memory buffer.
#[no_mangle]
unsafe extern "C" fn free(ptr: *mut u8, size: usize) {
    unsafe { std::alloc::dealloc(ptr, std::alloc::Layout::from_size_align(size, 1).unwrap()) }
}

pub unsafe fn into<'a>(str: &'a str) -> *const u8 {
    let bytes = str.as_bytes();
    let size = bytes.len().to_le_bytes();
    let ptr = std::alloc::alloc(std::alloc::Layout::from_size_align_unchecked(
        size.len() + bytes.len(),
        1,
    ));
    std::ptr::copy_nonoverlapping(size.as_ptr(), ptr, size.len());
    std::ptr::copy_nonoverlapping::<u8>(bytes.as_ptr(), ptr.offset(4), bytes.len());
    ptr
}
