//! Numbers in config files and packs must be finite. TOML reads `nan` and `inf`, but nothing
//! Padwight saves is one, and they break what's worked out from them: an overlay's `scale = nan`
//! draws the keyboard as one block over the whole screen.

use anyhow::{Result, bail};
use serde::de::DeserializeOwned;

/// Refuses `table` if any number in it, however deep, is `nan` or infinite, naming where.
pub fn check_finite(table: &toml::Table) -> Result<()> {
    match table.iter().find_map(|(key, value)| non_finite(value, key.clone())) {
        Some(at) => bail!("{at} isn't a number Padwight can use (nan or inf)"),
        None => Ok(()),
    }
}

/// Where in `value` (found at `at`) a number isn't finite, if anywhere.
fn non_finite(value: &toml::Value, at: String) -> Option<String> {
    match value {
        toml::Value::Float(f) if !f.is_finite() => Some(at),
        toml::Value::Array(items) => items.iter().enumerate().find_map(|(i, v)| non_finite(v, format!("{at}[{i}]"))),
        toml::Value::Table(table) => table.iter().find_map(|(key, v)| non_finite(v, format!("{at}.{key}"))),
        _ => None,
    }
}

/// Reads `text` as a `T`, refusing it if a number in it isn't finite (see [`check_finite`]).
pub(super) fn from_toml<T: DeserializeOwned>(text: &str) -> Result<T> {
    check_finite(&toml::from_str(text)?)?;
    Ok(toml::from_str(text)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nan_and_inf_are_refused_wherever_they_are() {
        let table = |text: &str| toml::from_str::<toml::Table>(text).unwrap();
        assert!(check_finite(&table("scale = 1.5\n[a]\nb = [1.0, 2]")).is_ok());
        let err = check_finite(&table("[menu_style]\nscale = nan")).unwrap_err().to_string();
        assert!(err.contains("menu_style.scale"), "{err}");
        let err = check_finite(&table("[[info_overlays]]\nlinger = inf")).unwrap_err().to_string();
        assert!(err.contains("info_overlays[0].linger"), "{err}");
        assert!(check_finite(&table("x = -inf")).is_err());
    }
}
