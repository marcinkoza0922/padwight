//! Sound files a pack carries: WAV files (PCM or float, any rate, any channels) decoded to the
//! mono samples at [`RATE`] every cue is made of, and the set of them the game in use has.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, RwLock},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::RATE;

/// The longest a decoded sound may be, in seconds: cues are short.
const MAX_SECONDS: f32 = 8.0;

/// The name of a sound file a pack carries. Names are interned, so a cue that uses one stays
/// `Copy`; there are as many as the packs in use name, which is few.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SoundFile(&'static str);

static NAMES: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());

impl SoundFile {
    pub fn new(name: &str) -> SoundFile {
        let mut names = NAMES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(known) = names.iter().find(|n| **n == name) {
            return SoundFile(known);
        }
        let leaked: &'static str = Box::leak(name.to_string().into_boxed_str());
        names.push(leaked);
        SoundFile(leaked)
    }
}

impl std::fmt::Debug for SoundFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SoundFile({:?})", self.0)
    }
}

impl std::fmt::Display for SoundFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "♪ {}", self.0)
    }
}

impl Serialize for SoundFile {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.0)
    }
}

impl<'de> Deserialize<'de> for SoundFile {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let name = String::deserialize(d)?;
        if crate::icons::valid_name(&name) { Ok(SoundFile::new(&name)) } else { Err(serde::de::Error::custom("a sound file's name is letters, digits, - and _")) }
    }
}

/// The sounds of the game in use, decoded, and the base64 they were decoded from.
static PACK: RwLock<Option<Loaded>> = RwLock::new(None);

/// The base64 the sounds came from, and the samples decoded from it, by name.
type Loaded = (BTreeMap<String, String>, BTreeMap<String, Arc<Vec<f32>>>);

/// Sets the sounds cues can name besides the built-in kinds: the active game's own. A sound
/// that doesn't decode is left out.
pub fn set_pack_sounds(sounds: &BTreeMap<String, String>) {
    if PACK.read().ok().and_then(|p| p.as_ref().map(|(from, _)| from == sounds)) == Some(true) {
        return;
    }
    let decoded = sounds
        .iter()
        .filter_map(|(name, data)| {
            let wav = crate::config::decode_base64(data)?;
            Some((name.clone(), Arc::new(decode_wav(&wav).ok()?)))
        })
        .collect();
    if let Ok(mut pack) = PACK.write() {
        *pack = Some((sounds.clone(), decoded));
    }
}

/// The names of the sounds the game in use carries, sorted.
pub fn pack_sound_names() -> Vec<SoundFile> {
    PACK.read().ok().and_then(|p| p.as_ref().map(|(_, d)| d.keys().map(|n| SoundFile::new(n)).collect())).unwrap_or_default()
}

/// The samples of sound `file`, if the game in use carries it.
pub fn pack_samples(file: SoundFile) -> Option<Arc<Vec<f32>>> {
    PACK.read().ok()?.as_ref()?.1.get(file.0).cloned()
}

/// A WAV file's format: its tag, channels, rate and bits per sample.
type Format = (u16, u16, u32, u16);

/// A WAV file's samples as mono at [`RATE`], at the file's own level.
pub fn decode_wav(bytes: &[u8]) -> Result<Vec<f32>, String> {
    let bad = |what: &str| format!("not a WAV file we can play ({what})");
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(bad("no RIFF header"));
    }
    let (mut format, mut data): (Option<Format>, Option<&[u8]>) = (None, None);
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let size = u32::from_le_bytes([bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]]) as usize;
        let body = &bytes[at + 8..(at + 8 + size).min(bytes.len())];
        match &bytes[at..at + 4] {
            b"fmt " if body.len() >= 16 => {
                let word = |i: usize| u16::from_le_bytes([body[i], body[i + 1]]);
                let mut tag = word(0);
                // Extensible files name the real format in their sub-format.
                if tag == 0xFFFE && body.len() >= 26 {
                    tag = word(24);
                }
                format = Some((tag, word(2), u32::from_le_bytes([body[4], body[5], body[6], body[7]]), word(14)));
            }
            b"data" => data = Some(body),
            _ => {}
        }
        at += 8 + size + (size & 1);
    }
    let (tag, channels, rate, bits) = format.ok_or_else(|| bad("no format"))?;
    let data = data.ok_or_else(|| bad("no sound"))?;
    if channels == 0 || rate == 0 {
        return Err(bad("empty format"));
    }
    let width = usize::from(bits / 8);
    let sample = |b: &[u8]| -> Option<f32> {
        Some(match (tag, bits) {
            (1, 8) => (f32::from(b[0]) - 128.0) / 128.0,
            (1, 16) => f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0,
            (1, 24) => (i32::from_le_bytes([0, b[0], b[1], b[2]]) >> 8) as f32 / 8_388_608.0,
            (1, 32) => i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32 / 2_147_483_648.0,
            (3, 32) => f32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            _ => return None,
        })
    };
    if !matches!((tag, bits), (1, 8 | 16 | 24 | 32) | (3, 32)) {
        return Err(bad("only PCM and float WAV files"));
    }
    let frame = width * usize::from(channels);
    let mono: Vec<f32> = data.chunks_exact(frame).map(|f| f.chunks_exact(width).filter_map(sample).sum::<f32>() / f32::from(channels)).collect();
    if mono.is_empty() {
        return Err(bad("no sound"));
    }
    if mono.len() as f32 / rate as f32 > MAX_SECONDS {
        return Err(format!("a sound is at most {MAX_SECONDS:.0} seconds long"));
    }
    Ok(resample(&mono, rate as f32 / RATE as f32))
}

/// `samples` read `step` samples at a time, which changes their pitch and length by `step`
/// (2 sounds an octave up and half as long). Linear, which is plenty for cues.
pub fn resample(samples: &[f32], step: f32) -> Vec<f32> {
    if (step - 1.0).abs() < 1e-4 {
        return samples.to_vec();
    }
    let count = (samples.len() as f32 / step).floor() as usize;
    (0..count)
        .map(|i| {
            let at = i as f32 * step;
            let (lo, frac) = (at as usize, at.fract());
            let next = samples.get(lo + 1).copied().unwrap_or(samples[lo]);
            samples[lo] * (1.0 - frac) + next * frac
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mono 16-bit WAV file of `samples` at `rate`.
    pub(crate) fn wav(samples: &[i16], rate: u32) -> Vec<u8> {
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut out = Vec::new();
        out.extend(b"RIFF");
        out.extend((36 + data.len() as u32).to_le_bytes());
        out.extend(b"WAVEfmt ");
        out.extend(16u32.to_le_bytes());
        out.extend([1, 0, 1, 0]);
        out.extend(rate.to_le_bytes());
        out.extend((rate * 2).to_le_bytes());
        out.extend([2, 0, 16, 0]);
        out.extend(b"data");
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
        out
    }

    #[test]
    fn a_wav_is_decoded_to_mono_at_the_cue_rate() {
        let same = decode_wav(&wav(&[0, 16384, -16384, 32767], RATE)).unwrap();
        assert_eq!(same.len(), 4);
        assert!((same[1] - 0.5).abs() < 1e-3 && (same[2] + 0.5).abs() < 1e-3);
        // Half the rate plays at twice the length.
        assert_eq!(decode_wav(&wav(&[0; 100], RATE / 2)).unwrap().len(), 200);
    }

    #[test]
    fn what_is_not_a_playable_wav_is_refused() {
        assert!(decode_wav(b"hello").is_err());
        assert!(decode_wav(&wav(&[], RATE)).is_err());
        let long = vec![0i16; (RATE as f32 * (MAX_SECONDS + 1.0)) as usize];
        assert!(decode_wav(&wav(&long, RATE)).is_err());
    }

    #[test]
    fn the_game_s_sounds_are_found_by_name() {
        let mut sounds = BTreeMap::new();
        sounds.insert("zap-test".to_string(), crate::config::encode_base64(&wav(&[0, 1000, -1000, 0], RATE)));
        set_pack_sounds(&sounds);
        assert_eq!(pack_samples(SoundFile::new("zap-test")).unwrap().len(), 4);
        assert!(pack_samples(SoundFile::new("nothing-test")).is_none());
        set_pack_sounds(&BTreeMap::new());
    }

    #[test]
    fn a_sound_file_s_name_survives_toml() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Holder {
            file: SoundFile,
        }
        let held = Holder { file: SoundFile::new("zap") };
        let text = toml::to_string(&held).unwrap();
        assert_eq!(toml::from_str::<Holder>(&text).unwrap(), held);
        assert!(toml::from_str::<Holder>("file = \"../x\"").is_err());
    }
}
