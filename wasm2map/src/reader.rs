use std::{borrow::Cow, ops::Deref};

#[derive(Clone, Debug)]
pub struct WasmReader<'a> {
    pub data: Cow<'a, [u8]>,
}

impl<'a> Deref for WasmReader<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.data.deref()
    }
}

unsafe impl<'a> gimli::StableDeref for WasmReader<'a> {}
unsafe impl<'a> gimli::CloneStableDeref for WasmReader<'a> {}
