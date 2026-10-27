//! Screen buffers in shared memory: one memfd, mapped once, with room for `slots::MOST`
//! buffers of one size, each reused once the compositor releases it (`slots.rs` keeps the
//! books). A frame is drawn straight into the buffer: when the compositor offers ABGR8888 that
//! is all; otherwise (KWin offers only ARGB8888 of the two) the bytes it repainted are then
//! swapped into ARGB8888's order in place (`render::swap_red_blue`).
//!
//! A pool is for one size and format. When either changes (hiding goes to 1×1, a new screen
//! size or scale) it is dropped and a new one made. Dropping destroys its buffers at once, even
//! ones the compositor still holds: wl_surface.attach allows destroying a buffer before its
//! release "as long as the underlying buffer storage isn't re-used", and it never is: the
//! compositor maps the memfd itself, and our mapping and fd go with the pool. So the memory
//! is ours to forget straight away, and the compositor frees its side when it is done.

use std::{
    fs::File,
    os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd},
    ptr::NonNull,
};

use anyhow::{Context, Result};
use wayland_client::{
    Dispatch, QueueHandle,
    protocol::{
        wl_buffer::WlBuffer,
        wl_shm::{self, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::WlSurface,
    },
};

use super::slots::{Layout, Lease, MOST, Slots};
use crate::overlay::render::Damage;

/// The layouts a frame can be handed over in, as the compositor offers them.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Formats {
    /// `ABGR8888`: bytes in memory R, G, B, A, as tiny-skia lays them out. Optional for a
    /// compositor to offer.
    pub(super) abgr: bool,
}

impl Formats {
    pub(super) fn offered(&mut self, format: wl_shm::Format) {
        if format == wl_shm::Format::Abgr8888 {
            self.abgr = true;
        }
    }

    /// The format buffers are made in. DRM formats name the bits of a little-endian word, high
    /// to low: ABGR8888 puts R in the lowest byte, so in memory it is R, G, B, A, which is
    /// tiny-skia's premultiplied RGBA, byte for byte. ARGB8888, which every compositor has, is
    /// B, G, R, A in memory. Both are premultiplied, as wl_shm's are.
    pub(super) fn format(self) -> wl_shm::Format {
        if self.abgr { wl_shm::Format::Abgr8888 } else { wl_shm::Format::Argb8888 }
    }
}

/// Which pool and buffer a `wl_buffer` is, for its `release`.
#[derive(Debug, Clone, Copy)]
pub(super) struct BufferId {
    pub(super) pool: u64,
    pub(super) slot: usize,
}

/// A pool of `MOST` buffers of one size, in one memfd.
pub(super) struct Pool {
    /// Which pool this is, so a release meant for an older one is passed over.
    pub(super) id: u64,
    layout: Layout,
    format: wl_shm::Format,
    map: Mapping,
    buffers: Vec<WlBuffer>,
    pub(super) slots: Slots,
}

impl Pool {
    pub(super) fn new<D>(shm: &WlShm, id: u64, layout: Layout, format: wl_shm::Format, qh: &QueueHandle<D>) -> Result<Pool>
    where
        D: Dispatch<WlShmPool, ()> + Dispatch<WlBuffer, BufferId> + 'static,
    {
        let len = layout.pool_len();
        let file = memfd(len)?;
        let map = Mapping::new(&file, len)?;
        let side = |n: usize| i32::try_from(n).context("the screen is too big for a buffer");
        let pool = shm.create_pool(file.as_fd(), side(len)?, qh, ());
        let buffers = (0..MOST)
            .map(|slot| {
                let offset = side(layout.offset(slot))?;
                let (w, h, stride) = (side(layout.width as usize)?, side(layout.height as usize)?, side(layout.stride())?);
                Ok(pool.create_buffer(offset, w, h, stride, format, qh, BufferId { pool: id, slot }))
            })
            .collect::<Result<Vec<_>>>()?;
        // The buffers keep the compositor's side of the memory; the pool object can go, and our
        // fd too: the mapping stays.
        pool.destroy();
        Ok(Pool { id, layout, format, map, buffers, slots: Slots::default() })
    }

    /// Whether this pool makes buffers of `layout` in `format`.
    pub(super) fn fits(&self, layout: Layout, format: wl_shm::Format) -> bool {
        fits((self.layout, self.format), (layout, format))
    }

    /// The pixels of the leased buffer, to draw into: never one the compositor holds.
    pub(super) fn pixels(&mut self, lease: Lease) -> &mut [u8] {
        self.slots.check_free(lease);
        let start = self.layout.offset(lease.slot);
        self.map.bytes_mut(start, self.layout.len())
    }

    /// Attaches the leased buffer to `surface` and damages what `damage` repainted of it, for
    /// the caller to commit.
    pub(super) fn attach(&mut self, lease: Lease, surface: &WlSurface, damage: &Damage) {
        surface.attach(Some(&self.buffers[lease.slot]), 0, 0);
        for p in damage.patches() {
            surface.damage_buffer(p.x0, p.y0, p.width(), p.height());
        }
        self.slots.committed(lease);
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        for buffer in &self.buffers {
            buffer.destroy();
        }
    }
}

/// Whether a pool made for `have` can give a buffer for `want`: only at the same size and
/// format. Hiding (1×1), a new screen size, and a new scale (a new size in device pixels) all
/// make a new pool.
fn fits(have: (Layout, wl_shm::Format), want: (Layout, wl_shm::Format)) -> bool {
    have == want
}

/// An anonymous shared-memory file of `len` bytes, sealed so its size can't change: then the
/// mapping can't lose its pages to someone shrinking the file.
fn memfd(len: usize) -> Result<File> {
    // SAFETY: memfd_create takes a NUL-terminated name and flags, and returns a new fd or -1.
    let fd = unsafe { libc::memfd_create(c"padwight-overlay".as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error()).context("memfd_create");
    }
    // SAFETY: the fd was just made and is owned by nothing else.
    let file = File::from(unsafe { OwnedFd::from_raw_fd(fd) });
    file.set_len(len as u64)?;
    let seals = libc::F_SEAL_SHRINK | libc::F_SEAL_GROW | libc::F_SEAL_SEAL;
    // SAFETY: fcntl on an fd we own, with an integer argument.
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_ADD_SEALS, seals) } < 0 {
        return Err(std::io::Error::last_os_error()).context("sealing the screen's memfd");
    }
    Ok(file)
}

/// A shared, writable mapping of a whole memfd, unmapped when dropped.
///
/// Sound to hand out as `&mut [u8]`: the mapping is ours alone in this process, borrowed only
/// through `&mut self`; the file is sealed, so its pages stay backed; and the other process
/// that maps it, the compositor, only reads a buffer, and only while it holds it, which
/// `Slots` keeps us from drawing into.
struct Mapping {
    ptr: NonNull<u8>,
    len: usize,
}

impl Mapping {
    fn new(file: &File, len: usize) -> Result<Mapping> {
        // SAFETY: a new shared mapping of `len` bytes of a file at least that long; the kernel
        // picks the address, so nothing else in this process is mapped there.
        let ptr = unsafe { libc::mmap(std::ptr::null_mut(), len, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_SHARED, file.as_raw_fd(), 0) };
        if ptr == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error()).context("mapping the screen's memfd");
        }
        let ptr = NonNull::new(ptr.cast()).context("mmap gave a null mapping")?;
        Ok(Mapping { ptr, len })
    }

    /// `len` bytes from `start`.
    fn bytes_mut(&mut self, start: usize, len: usize) -> &mut [u8] {
        assert!(start.checked_add(len).is_some_and(|end| end <= self.len), "{start}+{len} is outside the {}-byte mapping", self.len);
        // SAFETY: the range is inside the mapping, which lives as long as `self`, and `&mut
        // self` makes this the only borrow of it (see the type's notes).
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr().add(start), len) }
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        // SAFETY: the mapping made in `new`, unmapped once; no borrow of it outlives `self`.
        unsafe { libc::munmap(self.ptr.as_ptr().cast(), self.len) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiny_skias_bytes_are_abgr8888_and_convert_to_argb8888() {
        // A premultiplied half-opaque orange, drawn by tiny-skia.
        let mut pixmap = tiny_skia::Pixmap::new(1, 1).unwrap();
        pixmap.fill(tiny_skia::Color::from_rgba8(255, 128, 0, 128));
        let rgba = pixmap.data().to_vec();
        assert_eq!(rgba, [128, 64, 0, 128], "R, G, B, A in memory");
        // As ABGR8888 reads it: a little-endian word, A:B:G:R from the high bits down.
        let word = u32::from_le_bytes(rgba.clone().try_into().unwrap());
        assert_eq!((word >> 24, (word >> 16) & 0xff, (word >> 8) & 0xff, word & 0xff), (128, 0, 64, 128));
        let mut argb: [u8; 4] = rgba.try_into().unwrap();
        crate::overlay::render::swap_red_blue(&mut argb, 1, &Damage::full(1, 1));
        // ARGB8888: A:R:G:B from the high bits down.
        let word = u32::from_le_bytes(argb);
        assert_eq!((word >> 24, (word >> 16) & 0xff, (word >> 8) & 0xff, word & 0xff), (128, 128, 64, 0));
    }

    #[test]
    fn a_pool_is_remade_when_hiding_resizing_or_changing_format() {
        use wl_shm::Format::{Abgr8888, Argb8888};
        let screen = (Layout { width: 1920, height: 1080 }, Argb8888);
        assert!(fits(screen, screen), "frame after frame at one size reuses the pool");
        assert!(!fits(screen, (Layout { width: 1, height: 1 }, Argb8888)), "hidden: a 1×1 pool");
        assert!(!fits(screen, (Layout { width: 2880, height: 1620 }, Argb8888)), "a new size or scale");
        assert!(!fits(screen, (screen.0, Abgr8888)));
        assert_eq!(Formats { abgr: true }.format(), Abgr8888);
        assert_eq!(Formats::default().format(), Argb8888);
    }

    #[test]
    fn a_mapping_is_shared_with_its_file() {
        use std::os::unix::fs::FileExt;
        let file = memfd(4096).unwrap();
        let mut map = Mapping::new(&file, 4096).unwrap();
        map.bytes_mut(100, 4).copy_from_slice(&[1, 2, 3, 4]);
        let mut back = [0; 4];
        file.read_exact_at(&mut back, 100).unwrap();
        assert_eq!(back, [1, 2, 3, 4]);
        assert!(file.set_len(0).is_err(), "sealed against shrinking");
    }

    #[test]
    #[should_panic(expected = "outside")]
    fn a_range_past_the_mapping_is_refused() {
        let file = memfd(4096).unwrap();
        let mut map = Mapping::new(&file, 4096).unwrap();
        map.bytes_mut(4090, 8);
    }
}
