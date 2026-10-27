//! Profiling only: the overlay's timing log, for measuring and for scripts/overlay-harness.sh.

/// Profiling only. When `PADWIGHT_OVERLAY_TIMING` names a file, each received frame, each drawn
/// screen and each menu drawn are appended to it, as `recv <ns> <what>`, `built <ns> ...` and
/// `menu ...`, with the window's own lines (`present`, `commit`, `configure`, `surface`, `callback`,
/// `pool`: `wayland/window.rs`). docs/development/overlay-debugging.md lists their fields.
/// Unset, this does nothing; the file is opened per call, so it is only for measuring.
pub(super) fn timing_log(entry: &str) {
    use std::io::Write;
    let Some(path) = std::env::var_os("PADWIGHT_OVERLAY_TIMING") else { return };
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{entry}");
    }
}

/// What a received frame shows, for `timing_log`: its menu's title, cursor and picks, or its kind.
pub(super) fn frame_summary(frame: &super::OverlayFrame) -> String {
    match &frame.active {
        Some(super::OverlayView::Menu(m)) => format!("menu {:?} selected={:?} picks={}", m.title, m.selected, m.picks),
        Some(other) => crate::motion::Tracked::key(other),
        None => "none".into(),
    }
}

/// Profiling only, with `timing_log`: how a menu is drawn in this build, as
/// `menu <ns> <title> picks=… selected=… opacity=… lit=… flash=… since=…`, `since` giving
/// seconds since it opened, closed, its cursor moved and its last pick.
pub(super) fn menu_log(m: &crate::menu::MenuView, anim: &crate::motion::Anim) {
    if std::env::var_os("PADWIGHT_OVERLAY_TIMING").is_none() {
        return;
    }
    let t = &anim.timing;
    let lit = m.selected.map_or(0.0, |_| anim.lit(true, false));
    timing_log(&format!(
        "menu {} {:?} picks={} selected={:?} opacity={:.3} lit={:.2} flash={:.2} since=open:{:.3},closed:{:?},moved:{:.3},picked:{:.3}",
        unix_ns(),
        m.title,
        m.picks,
        m.selected,
        anim.opacity(),
        lit,
        anim.flash(),
        t.opened,
        t.closed.map(|c| (c * 1000.0).round() / 1000.0),
        t.moved,
        t.picked,
    ));
}

pub(super) fn unix_ns() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos())
}
