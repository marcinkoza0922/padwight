//! The pictures a user can add beside their config: icons for menu items and backgrounds for
//! overlays. Each kind has a folder under the padwight config folder (`icons/`, `backgrounds/`),
//! and a pack names a file in it by the file's name.

use std::path::{Path, PathBuf};

use crate::config::Config;

/// The folder the user keeps `kind` in: `padwight/<kind>`, beside `config.toml`.
pub fn folder(kind: &str) -> PathBuf {
    Config::path().parent().unwrap_or(Path::new(".")).join(kind)
}

/// Where `file` is: an absolute path is used as it is, and anything else is a file in `kind`'s
/// folder (a name with folders in it, `themes/crest.png`, is a path inside that folder).
pub fn locate(kind: &str, file: &str) -> PathBuf {
    let path = Path::new(file);
    if path.is_absolute() { path.to_path_buf() } else { folder(kind).join(path) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_relative_file_is_in_its_kind_s_folder_and_an_absolute_one_is_used_as_it_is() {
        assert_eq!(locate("backgrounds", "crest.png"), folder("backgrounds").join("crest.png"));
        assert_eq!(locate("backgrounds", "themes/crest.png"), folder("backgrounds").join("themes/crest.png"));
        let absolute = std::env::temp_dir().join("crest.png");
        assert_eq!(locate("backgrounds", &absolute.to_string_lossy()), absolute);
    }
}
