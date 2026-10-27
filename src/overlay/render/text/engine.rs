//! The text engine, one per thread (a cosmic-text font system can't be shared without a lock):
//! the fonts, the shaped runs by key, and the rasterized glyphs by glyph, size and sub-pixel bin.

use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::{DefaultHasher, Hash, Hasher},
    sync::{Arc, Mutex, PoisonError},
    thread::JoinHandle,
    time::Instant,
};

use cosmic_text::{CacheKey, FontSystem, SwashCache, SwashImage, fontdb::Source};

use super::{Canvas, Shaped, Spec, paint, set::Run};
use crate::{
    font,
    overlay::{
        render::{Point, Rgba, RunKey},
        timing::{timing_log, unix_ns},
    },
};

/// Past this many runs, those not used lately are dropped.
const RUNS: usize = 2048;

/// Past this many glyphs, those not used lately are dropped.
const GLYPHS: usize = 8192;

thread_local! {
    static ENGINE: RefCell<Option<Engine>> = const { RefCell::new(None) };
}

/// An engine being made on a thread of its own (`warm`), for the first thread that wants one.
static WARMING: Mutex<Option<JoinHandle<Engine>>> = Mutex::new(None);

/// Starts making an engine on a thread of its own, for the first thread to draw text to take.
/// Making one reads every installed font (some 3000 faces here): tens of milliseconds with them
/// in the page cache, seconds when they have to come off the disk, which would otherwise land on
/// the first frame drawn.
pub fn warm() {
    let mut warming = WARMING.lock().unwrap_or_else(PoisonError::into_inner);
    if warming.is_none() {
        *warming = std::thread::Builder::new().name("fonts".into()).spawn(Engine::new).ok();
    }
}

/// Runs `f` on this thread's engine, made on first use (or taken from `warm`, waited for if it
/// isn't ready), with its fonts brought up to date.
pub(super) fn with<R>(f: impl FnOnce(&mut Engine) -> R) -> R {
    ENGINE.with_borrow_mut(|engine| {
        let engine = engine.get_or_insert_with(|| {
            let warmed = WARMING.lock().unwrap_or_else(PoisonError::into_inner).take();
            warmed.and_then(|w| w.join().ok()).unwrap_or_else(Engine::new)
        });
        engine.sync_fonts();
        f(engine)
    })
}

pub(super) struct Engine {
    fonts: FontSystem,
    /// `font::version` when the fonts were last loaded; a run shaped before then is shaped again.
    version: u64,
    /// The files loaded, by `key`.
    files: HashSet<u64>,
    runs: Kept<RunKey, Run>,
    pub glyphs: Glyphs,
    /// How many runs were shaped rather than found, for the tests.
    #[cfg(test)]
    pub shaped: usize,
}

impl Engine {
    fn new() -> Engine {
        let started = Instant::now();
        let version = font::version();
        let files = font::files();
        let hashes = files.iter().map(key).collect();
        // The installed fonts too, as iced loads them: a setting can name any family.
        let fonts = FontSystem::new_with_fonts(files.into_iter().map(source));
        timing_log(&format!("fonts {} {} faces {}", unix_ns(), started.elapsed().as_nanos(), fonts.db().len()));
        Engine {
            fonts,
            version,
            files: hashes,
            runs: Kept::default(),
            glyphs: Glyphs { kept: Kept::default(), swash: SwashCache::new() },
            #[cfg(test)]
            shaped: 0,
        }
    }

    /// Loads the files `font` has added since last time (a game's fonts).
    fn sync_fonts(&mut self) {
        let version = font::version();
        if version == self.version {
            return;
        }
        for file in font::files() {
            if self.files.insert(key(&file)) {
                self.fonts.db_mut().load_font_source(source(file));
            }
        }
        self.version = version;
    }

    pub fn shape(&mut self, spec: &Spec) -> Shaped {
        let key = spec.key();
        if let Some(run) = self.runs.get(&key).filter(|run| run.spec == *spec && run.version == self.version) {
            return Shaped { run: key, size: run.size, ink: run.ink };
        }
        let run = self.build(spec.clone());
        let shaped = Shaped { run: key, size: run.size, ink: run.ink };
        self.runs.insert(key, run, RUNS);
        shaped
    }

    pub fn draw(&mut self, key: RunKey, at: Point, color: Rgba, canvas: &mut Canvas) {
        // Fonts loaded since it was shaped may change it, as they change what layout measures.
        if self.runs.get(&key).is_some_and(|run| run.version != self.version) {
            let spec = self.runs.get(&key).map(|run| run.spec.clone());
            if let Some(spec) = spec {
                let run = self.build(spec);
                self.runs.insert(key, run, RUNS);
            }
        }
        let Engine { runs, glyphs, fonts, .. } = self;
        if let Some(run) = runs.get(&key) {
            paint::run(run, &mut glyphs.with(fonts), at, color, canvas);
        }
    }

    fn build(&mut self, spec: Spec) -> Run {
        #[cfg(test)]
        {
            self.shaped += 1;
        }
        let mut run = Run::shape(&mut self.fonts, spec, &mut self.glyphs);
        run.version = self.version;
        run
    }
}

fn source(file: Cow<'static, [u8]>) -> Source {
    match file {
        Cow::Borrowed(bytes) => Source::Binary(Arc::new(bytes)),
        Cow::Owned(bytes) => Source::Binary(Arc::new(bytes)),
    }
}

/// Tells font files apart: a bundled one by where it is, as hashing would read every byte of it
/// (and so keep it all in memory), a game's by a hash of its bytes.
#[expect(clippy::ptr_arg, reason = "whether it is borrowed is what tells a bundled font")]
fn key(file: &Cow<'static, [u8]>) -> u64 {
    let mut hasher = DefaultHasher::new();
    match file {
        Cow::Borrowed(bytes) => (bytes.as_ptr() as usize, bytes.len()).hash(&mut hasher),
        Cow::Owned(bytes) => bytes.hash(&mut hasher),
    }
    hasher.finish()
}

/// Rasterized glyphs, as swash draws them: coverage masks, or color for color fonts.
pub(super) struct Glyphs {
    kept: Kept<CacheKey, Option<SwashImage>>,
    swash: SwashCache,
}

impl Glyphs {
    /// These glyphs with the fonts to draw new ones from.
    pub fn with<'a>(&'a mut self, fonts: &'a mut FontSystem) -> GlyphSource<'a> {
        GlyphSource { glyphs: self, fonts }
    }
}

/// The glyph cache and the fonts, borrowed together.
pub(super) struct GlyphSource<'a> {
    glyphs: &'a mut Glyphs,
    fonts: &'a mut FontSystem,
}

impl GlyphSource<'_> {
    /// The glyph's picture, drawn the first time it's asked for; `None` for one with no ink.
    pub fn get(&mut self, key: CacheKey) -> Option<&SwashImage> {
        let Glyphs { kept, swash } = &mut *self.glyphs;
        if kept.get(&key).is_none() {
            let image = swash.get_image_uncached(self.fonts, key).filter(|i| i.placement.width > 0 && i.placement.height > 0);
            kept.insert(key, image, GLYPHS);
        }
        kept.get(&key)?.as_ref()
    }
}

/// A map that drops what wasn't used lately once it holds `cap` entries: what was used since
/// the drop before last stays, so a run shaped for a frame is still there when it is drawn.
struct Kept<K, V> {
    entries: HashMap<K, (V, u64)>,
    /// Goes up at each drop.
    epoch: u64,
}

impl<K, V> Default for Kept<K, V> {
    fn default() -> Self {
        Kept { entries: HashMap::new(), epoch: 1 }
    }
}

impl<K: Eq + Hash, V> Kept<K, V> {
    fn get(&mut self, key: &K) -> Option<&V> {
        let epoch = self.epoch;
        self.entries.get_mut(key).map(|(value, used)| {
            *used = epoch;
            &*value
        })
    }

    fn insert(&mut self, key: K, value: V, cap: usize) {
        if self.entries.len() >= cap && !self.entries.contains_key(&key) {
            let epoch = self.epoch;
            self.entries.retain(|_, (_, used)| *used + 1 >= epoch);
            self.epoch += 1;
        }
        self.entries.insert(key, (value, self.epoch));
    }
}

#[cfg(test)]
impl Engine {
    pub fn holds(&self, key: RunKey) -> bool {
        self.runs.entries.contains_key(&key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_was_used_lately_is_kept_past_the_cap() {
        let mut kept = Kept::default();
        for i in 0..4 {
            kept.insert(i, i, 4);
        }
        kept.insert(4, 4, 4);
        assert_eq!(kept.entries.len(), 5, "all used since the last drop, so all kept");
        assert!(kept.get(&4).is_some());
        kept.insert(5, 5, 4);
        kept.insert(6, 6, 4);
        assert!(kept.get(&0).is_none(), "unused for two drops");
        assert!(kept.get(&4).is_some() && kept.get(&6).is_some());
    }
}
