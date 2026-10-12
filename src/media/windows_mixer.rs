//! The system's sound on Windows: the default output's volume and mute, through its endpoint
//! volume. The media controls change it with the D-pad.

use windows::Win32::{
    Media::Audio::{
        Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender,
    },
    System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx},
};

/// The default output's volume (0 to 1) and whether it's muted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    pub volume: f32,
    pub muted: bool,
}

/// The default output's volume control, or `None` without an output.
fn endpoint() -> Option<IAudioEndpointVolume> {
    // SAFETY: COM calls on this thread, initialized first; the objects are reference counted.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let device = devices.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
        device.Activate(CLSCTX_ALL, None).ok()
    }
}

/// The default output's level, or `None` without an output.
pub fn read() -> Option<Level> {
    let volume = endpoint()?;
    // SAFETY: plain reads of a live COM object.
    unsafe { Some(Level { volume: volume.GetMasterVolumeLevelScalar().ok()?, muted: volume.GetMute().ok()?.as_bool() }) }
}

/// Changes the default output's volume by `delta` (a share of full, negative to lower it), up to
/// full. False without an output.
pub fn change(delta: f64) -> bool {
    let Some(volume) = endpoint() else { return false };
    // SAFETY: plain calls on a live COM object.
    unsafe {
        let Ok(now) = volume.GetMasterVolumeLevelScalar() else { return false };
        let to = (f64::from(now) + delta).clamp(0.0, 1.0) as f32;
        volume.SetMasterVolumeLevelScalar(to, std::ptr::null()).is_ok()
    }
}

/// Mutes the default output, or unmutes it. False without an output.
pub fn toggle_mute() -> bool {
    let Some(volume) = endpoint() else { return false };
    // SAFETY: plain calls on a live COM object.
    unsafe {
        let Ok(muted) = volume.GetMute() else { return false };
        volume.SetMute(!muted.as_bool(), std::ptr::null()).is_ok()
    }
}
