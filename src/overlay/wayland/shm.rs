//! Screen buffers in shared memory. For now each frame gets a new memfd and buffer, freed once
//! the compositor releases it; step 5.2 replaces this with a pool of two or three reused ones,
//! drawn into directly.

use std::{
    fs::File,
    os::{
        fd::{AsFd, FromRawFd, OwnedFd},
        unix::fs::FileExt,
    },
};

use anyhow::{Context, Result};
use wayland_client::{
    QueueHandle,
    protocol::{wl_buffer::WlBuffer, wl_shm},
};

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
}

/// Puts `pixels` (premultiplied RGBA, `width`×`height`) into a new shared-memory buffer.
pub(super) fn buffer<D>(shm: &wl_shm::WlShm, formats: Formats, (width, height): (u32, u32), pixels: &[u8], qh: &QueueHandle<D>) -> Result<WlBuffer>
where
    D: wayland_client::Dispatch<wayland_client::protocol::wl_shm_pool::WlShmPool, ()> + wayland_client::Dispatch<WlBuffer, ()> + 'static,
{
    let stride = width * 4;
    let len = stride as usize * height as usize;
    debug_assert_eq!(pixels.len(), len);
    let file = memfd(len)?;
    // ARGB8888, which every compositor has, is B, G, R, A in memory.
    let format = if formats.abgr {
        file.write_all_at(pixels, 0)?;
        wl_shm::Format::Abgr8888
    } else {
        let mut bgra = pixels.to_vec();
        for px in bgra.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
        }
        file.write_all_at(&bgra, 0)?;
        wl_shm::Format::Argb8888
    };
    let pool = shm.create_pool(file.as_fd(), i32::try_from(len).context("the screen is too big for a buffer")?, qh, ());
    let buffer = pool.create_buffer(0, width as i32, height as i32, stride as i32, format, qh, ());
    // The buffer keeps the memory; the pool and our file can go.
    pool.destroy();
    Ok(buffer)
}

/// An anonymous shared-memory file of `len` bytes.
fn memfd(len: usize) -> Result<File> {
    // SAFETY: memfd_create takes a NUL-terminated name and flags, and returns a new fd or -1.
    let fd = unsafe { libc::memfd_create(c"padwight-overlay".as_ptr(), libc::MFD_CLOEXEC) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error()).context("memfd_create");
    }
    // SAFETY: the fd was just made and is owned by nothing else.
    let file = File::from(unsafe { OwnedFd::from_raw_fd(fd) });
    file.set_len(len as u64)?;
    Ok(file)
}
