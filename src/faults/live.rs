//! The working half of [`super`], built into test and debug builds.

use std::{
    io,
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use super::Action;

struct Rule {
    point: String,
    action: Action,
    skip: u64,
    fire: u64,
    of: u64,
    hits: u64,
}

impl Rule {
    fn matches(&self, point: &str) -> bool {
        match self.point.strip_suffix('*') {
            Some(prefix) => point.starts_with(prefix),
            None => self.point == point,
        }
    }
}

static ARMED: AtomicBool = AtomicBool::new(false);
static RULES: Mutex<Vec<Rule>> = Mutex::new(Vec::new());
/// Held by [`set`]'s guard, so tests that inject faults in one process run one at a time.
#[cfg(test)]
static SERIAL: Mutex<()> = Mutex::new(());

fn rules() -> MutexGuard<'static, Vec<Rule>> {
    RULES.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// A parsed rule: point, action, hits to skip, and the N and M of `:N/M`.
pub type Parsed = (String, Action, u64, u64, u64);

/// Parses a spec (see the module above). An empty spec is no rules.
pub fn parse(spec: &str) -> Result<Vec<Parsed>, String> {
    let mut out = Vec::new();
    for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (point, what) = part.split_once('=').ok_or_else(|| format!("“{part}” has no ="))?;
        let (what, ratio) = what.split_once(':').map_or((what, None), |(w, r)| (w, Some(r)));
        let (name, skip) = what.split_once('+').map_or((what, "0"), |(n, s)| (n, s));
        let action = match name {
            "fail" => Action::Fail,
            "hang" => Action::Hang,
            "close" => Action::Close,
            "crash" => Action::Crash,
            other => return Err(format!("“{other}” is not fail, hang, close or crash")),
        };
        let number = |s: &str| s.parse::<u64>().map_err(|_| format!("“{s}” in “{part}” is not a number"));
        let (fire, of) = match ratio.map(|r| r.split_once('/')) {
            None => (1, 1),
            Some(Some((n, m))) => (number(n)?, number(m)?),
            Some(None) => return Err(format!("“{part}”: write the ratio as N/M")),
        };
        if of == 0 {
            return Err(format!("“{part}”: M can't be 0"));
        }
        out.push((point.to_string(), action, number(skip)?, fire, of));
    }
    Ok(out)
}

fn install(spec: &str) -> Result<(), String> {
    let parsed = parse(spec)?;
    let mut rules = rules();
    *rules = parsed.into_iter().map(|(point, action, skip, fire, of)| Rule { point, action, skip, fire, of, hits: 0 }).collect();
    ARMED.store(!rules.is_empty(), Ordering::SeqCst);
    Ok(())
}

/// Reads `PADWIGHT_FAULTS`. The daemon calls this when started with `--debug`.
pub fn arm() {
    let Ok(spec) = std::env::var("PADWIGHT_FAULTS") else { return };
    match install(&spec) {
        Ok(()) => crate::monitor::log!("faults: armed with {spec}"),
        Err(e) => crate::monitor::log!("faults: ignoring PADWIGHT_FAULTS, {e}"),
    }
}

/// Holds a spec in place until dropped, and keeps other tests from setting theirs meanwhile.
#[cfg(test)]
pub struct Guard(#[expect(dead_code, reason = "held for its lock")] MutexGuard<'static, ()>);

#[cfg(test)]
impl Drop for Guard {
    fn drop(&mut self) {
        rules().clear();
        ARMED.store(false, Ordering::SeqCst);
    }
}

/// Sets the faults to `spec` until the guard is dropped.
///
/// # Panics
/// When the spec doesn't parse.
#[cfg(test)]
pub fn set(spec: &str) -> Guard {
    let guard = Guard(SERIAL.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
    install(spec).unwrap_or_else(|e| panic!("bad fault spec: {e}"));
    guard
}

/// What the point `point` does on this hit, if a rule fires. Hangs and crashes happen in here.
pub fn hit(point: &str) -> Option<Action> {
    if !ARMED.load(Ordering::Relaxed) {
        return None;
    }
    let action = {
        let mut rules = rules();
        let rule = rules.iter_mut().find(|r| r.matches(point))?;
        rule.hits += 1;
        let n = rule.hits.checked_sub(rule.skip + 1)?;
        (n % rule.of < rule.fire).then_some(rule.action)?
    };
    match action {
        Action::Crash => std::process::abort(),
        Action::Hang => loop {
            thread::sleep(Duration::from_secs(3600));
        },
        other => Some(other),
    }
}

/// A point on an I/O call: an error when it fires with `fail` or `close`.
pub fn io(point: &str) -> io::Result<()> {
    match hit(point) {
        Some(_) => Err(io::Error::other(format!("injected fault at {point}"))),
        None => Ok(()),
    }
}
