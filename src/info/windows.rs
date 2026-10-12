//! Live values on Windows: the local time, and system load from the kernel's own counters.

use std::sync::OnceLock;

use windows::{
    Win32::{
        Foundation::FILETIME,
        System::{
            Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
            Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW},
            SystemInformation::{GetLocalTime, GlobalMemoryStatusEx, MEMORYSTATUSEX},
            Threading::GetSystemTimes,
        },
    },
    core::{HSTRING, w},
};

use super::{Charge, FormFactor, detect_form};

const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// Formats the local time with the `strftime` patterns the info overlays use: `%H %M %S %I %p
/// %a %d %b %m %Y %%`, with `-` for no padding (`%-I`). Names are English, as strftime's are in
/// the C locale Linux uses.
pub(super) fn local_time(pattern: &str) -> String {
    // SAFETY: GetLocalTime has no preconditions.
    let t = unsafe { GetLocalTime() };
    let hour12 = match t.wHour % 12 {
        0 => 12,
        h => h,
    };
    let mut out = String::new();
    let mut chars = pattern.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let mut spec = chars.next();
        let padded = spec != Some('-');
        if !padded {
            spec = chars.next();
        }
        let number = |n: u16| if padded { format!("{n:02}") } else { n.to_string() };
        match spec {
            Some('H') => out += &number(t.wHour),
            Some('M') => out += &number(t.wMinute),
            Some('S') => out += &number(t.wSecond),
            Some('I') => out += &number(hour12),
            Some('p') => out += if t.wHour < 12 { "AM" } else { "PM" },
            Some('a') => out += DAYS[usize::from(t.wDayOfWeek % 7)],
            Some('d') => out += &number(t.wDay),
            Some('b') => out += MONTHS[usize::from(t.wMonth.clamp(1, 12) - 1)],
            Some('m') => out += &number(t.wMonth),
            Some('Y') => out += &t.wYear.to_string(),
            Some('%') => out.push('%'),
            Some(other) => {
                out.push('%');
                out.push(other);
            }
            None => out.push('%'),
        }
    }
    out
}

fn ticks(time: FILETIME) -> u64 {
    (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime)
}

/// Busy and total time across all cores, in 100 ns ticks.
pub(super) fn cpu_times() -> Option<(u64, u64)> {
    let (mut idle, mut kernel, mut user) = Default::default();
    // SAFETY: each pointer is to a local FILETIME.
    unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.ok()?;
    // Kernel time includes the idle time.
    let total = ticks(kernel) + ticks(user);
    Some((total - ticks(idle), total))
}

/// Used and total memory, in GB.
pub(super) fn memory() -> Option<(f32, f32)> {
    let mut status = MEMORYSTATUSEX { dwLength: size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    // SAFETY: the structure has its size set, as the API asks.
    unsafe { GlobalMemoryStatusEx(&mut status) }.ok()?;
    let gb = 1024.0 * 1024.0 * 1024.0;
    Some(((status.ullTotalPhys - status.ullAvailPhys) as f32 / gb, status.ullTotalPhys as f32 / gb))
}

/// Windows has no simple file to read GPU load from.
pub(super) fn gpu_busy() -> Option<f32> {
    None
}

fn power() -> Option<SYSTEM_POWER_STATUS> {
    let mut status = SYSTEM_POWER_STATUS::default();
    // SAFETY: the structure is filled in by the call.
    unsafe { GetSystemPowerStatus(&mut status) }.ok()?;
    Some(status)
}

/// The system battery's charge, when there is one.
pub(super) fn battery() -> Option<Charge> {
    const NO_BATTERY: u8 = 128;
    let status = power()?;
    (status.BatteryFlag & NO_BATTERY == 0 && status.BatteryLifePercent <= 100)
        .then_some(Charge { percent: status.BatteryLifePercent, estimated: false })
}

/// Windows has no simple file to read Wi-Fi signal from.
pub(super) fn wifi_strength() -> Option<u8> {
    None
}

/// A value from the BIOS's own description of the machine, in the registry.
fn bios(name: &HSTRING) -> String {
    let mut buf = [0u16; 256];
    let mut len = (buf.len() * 2) as u32;
    // SAFETY: the buffer is as long as the size passed, in bytes.
    let found = unsafe {
        RegGetValueW(HKEY_LOCAL_MACHINE, w!(r"HARDWARE\DESCRIPTION\System\BIOS"), name, RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr().cast()), Some(&mut len))
    };
    if found.is_err() {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..(len as usize / 2).saturating_sub(1)])
}

/// The form factor: handhelds by their product and vendor names, and otherwise a laptop if it has
/// a battery. Windows doesn't keep the chassis type anywhere simple to read.
pub fn form_factor() -> FormFactor {
    static FORM: OnceLock<FormFactor> = OnceLock::new();
    *FORM.get_or_init(|| {
        // SMBIOS chassis types: 9 is a laptop, 3 a desktop.
        let chassis = if battery().is_some() { "9" } else { "3" };
        detect_form(bios(&HSTRING::from("SystemProductName")).trim(), bios(&HSTRING::from("SystemManufacturer")).trim(), chassis)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_follow_the_patterns() {
        let time = local_time("%H:%M");
        assert_eq!(time.len(), 5, "{time}");
        let date = local_time("%a %-d %b %Y");
        assert!(DAYS.iter().any(|d| date.starts_with(d)), "{date}");
        assert!(local_time("%-I:%M %p").ends_with('M'));
        assert_eq!(local_time("100%%"), "100%");
    }

    #[test]
    fn system_load_is_read() {
        let (busy, total) = cpu_times().unwrap();
        assert!(busy <= total);
        let (used, all) = memory().unwrap();
        assert!(used > 0.0 && used <= all);
    }
}
