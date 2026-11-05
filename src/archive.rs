//! Files compressed into the binary by `build.rs`: the game library, and the icons, fonts and
//! sounds. An [`Archive`] holds each file packed on its own (zstd), so opening one costs only
//! that file, and the library packs share a dictionary trained on all of them, which is what
//! makes thousands of near-identical packs small.

use std::sync::OnceLock;

use zstd::{bulk::Decompressor, dict::DecoderDictionary};

/// Where one file sits in an archive, as `build.rs` wrote it.
pub struct Item {
    pub name: &'static str,
    /// Its place in the blob and its packed length.
    pub at: u32,
    pub packed: u32,
    /// Its length unpacked.
    pub raw: u32,
}

pub struct Archive {
    blob: &'static [u8],
    /// Empty when the files were packed without one.
    dict: &'static [u8],
    items: &'static [Item],
    prepared: OnceLock<DecoderDictionary<'static>>,
}

impl Archive {
    pub const fn new(blob: &'static [u8], dict: &'static [u8], items: &'static [Item]) -> Self {
        Archive { blob, dict, items, prepared: OnceLock::new() }
    }

    pub fn items(&self) -> &'static [Item] {
        self.items
    }

    /// The `i`th file, unpacked.
    pub fn unpack(&self, i: usize) -> Vec<u8> {
        let item = &self.items[i];
        let packed = &self.blob[item.at as usize..(item.at + item.packed) as usize];
        let mut decoder = if self.dict.is_empty() {
            Decompressor::new()
        } else {
            Decompressor::with_prepared_dictionary(self.prepared.get_or_init(|| DecoderDictionary::copy(self.dict)))
        }
        .expect("a decompressor");
        decoder.decompress(packed, item.raw as usize).unwrap_or_else(|e| panic!("{} is damaged in the binary: {e}", item.name))
    }

    /// The `i`th file as text; every archived text file is UTF-8 (`build.rs` read it as such).
    pub fn text(&self, i: usize) -> String {
        String::from_utf8(self.unpack(i)).expect("archived text is UTF-8")
    }

    /// The place of the file called `name`.
    pub fn find(&self, name: &str) -> Option<usize> {
        self.items.iter().position(|item| item.name == name)
    }

    /// The file called `name`, unpacked.
    pub fn get(&self, name: &str) -> Option<Vec<u8>> {
        self.find(name).map(|i| self.unpack(i))
    }
}

/// The icons, fonts and sounds (`assets/`), named by their path under it (`fonts/Inter-Regular.ttf`).
pub mod assets {
    use super::{Archive, Item};
    include!(concat!(env!("OUT_DIR"), "/assets.rs"));
    pub static ARCHIVE: Archive = Archive::new(BLOB, DICT, ITEMS);
}
