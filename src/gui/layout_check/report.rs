//! The findings as text: the same problem in several scenes or themes is one line.

use std::fmt::Write;

use iced::Size;

use super::{Finding, Kind};

/// A problem with where it was seen.
pub struct Unique<'a> {
    pub kind: Kind,
    pub text: &'a str,
    pub detail: &'a str,
    pub seen: Vec<String>,
}

/// The findings with repeats merged, worst kind first, then most widespread first.
pub fn unique(findings: &[Finding]) -> Vec<Unique<'_>> {
    let mut out: Vec<Unique<'_>> = Vec::new();
    for f in findings {
        let place = format!("{} {}", f.scene, f.theme);
        // Contrast differs a little wherever a text sits; the worst one stands for them all.
        let same = |u: &Unique<'_>| u.kind == f.kind && u.text == f.text && (u.detail == f.detail || f.kind == Kind::LowContrast);
        match out.iter_mut().find(|u| same(u)) {
            Some(u) => {
                if f.kind == Kind::LowContrast && ratio_of(&f.detail) < ratio_of(u.detail) {
                    u.detail = &f.detail;
                }
                u.seen.push(place);
            }
            None => out.push(Unique { kind: f.kind, text: &f.text, detail: &f.detail, seen: vec![place] }),
        }
    }
    out.sort_by(|a, b| a.kind.cmp(&b.kind).then(b.seen.len().cmp(&a.seen.len())));
    out
}

fn ratio_of(detail: &str) -> f32 {
    detail.split(':').next().and_then(|r| r.parse().ok()).unwrap_or(f32::MAX)
}

pub fn text(findings: &[Finding], skipped: &[String], states: usize, size: Size) -> String {
    let all = unique(findings);
    let mut s = String::new();
    let _ = writeln!(s, "GUI layout check: {states} states at {}x{}, {} distinct findings", size.width, size.height, all.len());
    let mut last = None;
    for u in &all {
        if last != Some(u.kind) {
            let n = all.iter().filter(|o| o.kind == u.kind).count();
            let _ = writeln!(s, "\n== {} ({n})", u.kind.name());
            last = Some(u.kind);
        }
        let shown: Vec<_> = u.seen.iter().take(3).map(String::as_str).collect();
        let more = u.seen.len().saturating_sub(3);
        let _ = writeln!(s, "  {:?}: {}\n      in {}{}", u.text, u.detail, shown.join("; "), if more > 0 { format!("; and {more} more") } else { String::new() });
    }
    for line in skipped {
        let _ = writeln!(s, "\nskipped: {line}");
    }
    s
}
