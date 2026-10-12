//! The files a pack carries: icons for `{icon:name}` and sound cues for the overlays. They live
//! in the game as text (an SVG as it is, a WAV as base64), so a pack stays one file and a game
//! exports with everything it uses. They are the game's own: icons and sounds beyond the ones
//! built in work only while this game is in use.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The most a sound file may be, in bytes: cues are short.
pub const MAX_SOUND_BYTES: usize = 600 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PackAssets {
    /// SVG markup by name, drawn in the text's color where it says `currentColor`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub icons: BTreeMap<String, String>,
    /// WAV files by name, as base64.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub sounds: BTreeMap<String, String>,
    /// TrueType or OpenType font files by name, as base64.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub fonts: BTreeMap<String, String>,
}

impl PackAssets {
    pub fn is_empty(&self) -> bool {
        self.icons.is_empty() && self.sounds.is_empty() && self.fonts.is_empty()
    }

    /// Makes these the icons and sounds in use: what `{icon:…}` and the cues' sound files can
    /// name besides the built-in ones.
    pub fn activate(&self) {
        crate::icons::set_pack_icons(&self.icons);
        crate::sound::set_pack_sounds(&self.sounds);
        crate::font::set_pack_fonts(&self.fonts);
    }

    /// Like [`PackAssets::activate`], for the daemon, which draws nothing: it keeps the sounds,
    /// and shares the fonts as files the overlay loads, returning where they are.
    pub fn activate_headless(&self) -> BTreeMap<String, std::path::PathBuf> {
        crate::icons::set_pack_icons(&self.icons);
        crate::sound::set_pack_sounds(&self.sounds);
        crate::font::share_pack_fonts(&self.fonts)
    }

    /// Adds (or replaces) font `name` from the bytes of a TrueType or OpenType file.
    pub fn add_font(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        if !crate::font::valid_name(name) {
            return Err("a name is letters, digits, spaces, - _ and .".into());
        }
        crate::font::load(bytes)?;
        self.fonts.insert(name.to_string(), encode_base64(bytes));
        Ok(())
    }

    /// Adds (or replaces) icon `name`.
    pub fn add_icon(&mut self, name: &str, svg: &str) -> Result<(), String> {
        check_name(name)?;
        crate::icons::check_svg(svg)?;
        self.icons.insert(name.to_string(), svg.to_string());
        Ok(())
    }

    /// Adds (or replaces) sound `name` from the bytes of a WAV file.
    pub fn add_sound(&mut self, name: &str, wav: &[u8]) -> Result<(), String> {
        check_name(name)?;
        if wav.len() > MAX_SOUND_BYTES {
            return Err(format!("a sound is at most {} KB", MAX_SOUND_BYTES / 1024));
        }
        crate::sound::decode_wav(wav)?;
        self.sounds.insert(name.to_string(), encode_base64(wav));
        Ok(())
    }

    /// Whether everything in these assets can be used: names are plain, icons are clean SVG and
    /// sounds are WAV files. The first problem found, said for the person importing the pack.
    pub fn check(&self) -> Result<(), String> {
        for (name, svg) in &self.icons {
            check_name(name).and_then(|()| crate::icons::check_svg(svg)).map_err(|e| format!("icon “{name}”: {e}"))?;
        }
        for (name, data) in &self.fonts {
            let font = decode_base64(data).ok_or_else(|| "not base64".to_string());
            let valid = if crate::font::valid_name(name) { Ok(()) } else { Err("a name is letters, digits, spaces, - _ and .".to_string()) };
            valid.and(font).and_then(|bytes| crate::font::load(&bytes).map(drop)).map_err(|e| format!("font “{name}”: {e}"))?;
        }
        for (name, data) in &self.sounds {
            let decoded = check_name(name).and_then(|()| decode_base64(data).ok_or_else(|| "not base64".to_string()));
            decoded
                .and_then(|wav| if wav.len() > MAX_SOUND_BYTES { Err("too big".to_string()) } else { crate::sound::decode_wav(&wav).map(drop) })
                .map_err(|e| format!("sound “{name}”: {e}"))?;
        }
        Ok(())
    }
}

fn check_name(name: &str) -> Result<(), String> {
    if crate::icons::valid_name(name) { Ok(()) } else { Err("a name is letters, digits, - and _".into()) }
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn encode_base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The bytes `data` says, ignoring whitespace; `None` if it isn't base64.
pub fn decode_base64(data: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len() / 4 * 3);
    let (mut bits, mut have) = (0u32, 0);
    for c in data.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=') {
        let v = ALPHABET.iter().position(|a| *a == c)? as u32;
        bits = bits << 6 | v;
        have += 6;
        if have >= 8 {
            have -= 8;
            out.push((bits >> have & 0xff) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips() {
        for bytes in [&b""[..], b"f", b"fo", b"foo", b"foob", &[0, 255, 128, 7, 9]] {
            assert_eq!(decode_base64(&encode_base64(bytes)).unwrap(), bytes);
        }
        assert_eq!(encode_base64(b"foobar"), "Zm9vYmFy");
        assert!(decode_base64("a$b").is_none());
    }

    #[test]
    fn assets_are_named_plainly_and_checked() {
        let mut assets = PackAssets::default();
        assert!(assets.add_icon("bow-2", "<svg viewBox='0 0 8 8'></svg>").is_ok());
        assert!(assets.add_icon("../x", "<svg></svg>").is_err());
        assert!(assets.add_icon("bad", "<p>hi</p>").is_err());
        assert!(assets.add_sound("zap", b"not a wav").is_err());
        assert!(assets.check().is_ok() && !assets.is_empty());
    }
}
