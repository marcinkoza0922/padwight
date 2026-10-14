//! The game's named items (macros, menus, overlays, modes and layers): renaming one, which every
//! action that names it follows, and deleting one, which nothing may still use.

use crate::{
    config::{Config, ItemKind, free_name},
    pack::game_references,
};

use super::{owner::game_mut, rows::Row};

/// The name of the game's item `i` of `kind`.
pub(super) fn name(config: &Config, kind: ItemKind, i: usize) -> Option<String> {
    config.active_game().names(kind).get(i).map(|n| (*n).to_string())
}

/// Whether anything in the game uses its item `i` of `kind`.
pub(super) fn in_use(config: &Config, kind: ItemKind, i: usize) -> bool {
    let game = config.active_game();
    name(config, kind, i).is_some_and(|n| game_references(game).contains(&(kind, n)))
}

/// A name no other item of `kind` in the game has: `base 1`, `base 2`, …
pub(super) fn new_name(config: &Config, kind: ItemKind, base: &str) -> String {
    let names = config.active_game().names(kind);
    (1..).map(|n| format!("{base} {n}")).find(|n| !names.contains(&n.as_str())).unwrap_or_default()
}

/// Renames item `i` of `kind` to `typed` (trimmed; a name another item has gets a number), and
/// points everything that named it at the new name. False when nothing changed.
pub(super) fn rename(config: &mut Config, kind: ItemKind, i: usize, typed: &str) -> bool {
    let typed = typed.trim();
    let Some(old) = name(config, kind, i) else { return false };
    if typed.is_empty() || typed == old {
        return false;
    }
    let Some(game) = game_mut(config) else { return false };
    let names: Vec<String> = game.names(kind).iter().map(|n| (*n).to_string()).collect();
    let new = free_name(typed, |n| names.iter().any(|x| x == n));
    game.rename_item(kind, &old, &new);
    game.rename_refs(kind, &old, &new);
    true
}

/// The row that renames item `i` of `kind`.
pub(super) fn rename_row(config: &Config, kind: ItemKind, i: usize) -> Row {
    let now = name(config, kind, i).unwrap_or_default();
    Row::text(format!("Name: {now}"), format!("Name of the {}", kind.noun()), now, move |c, typed| rename(c, kind, i, typed))
}

/// The row that deletes item `i` of `kind` with `delete`, or says why it can't.
pub(super) fn delete_row(config: &Config, kind: ItemKind, i: usize, delete: impl Fn(&mut Config) -> bool + 'static) -> Row {
    if in_use(config, kind, i) {
        return Row::note(format!("In use: this {} can't be deleted", kind.noun()));
    }
    Row::delete(format!("Delete {}", kind.noun()), delete)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ButtonAction, Macro};

    #[test]
    fn a_rename_reaches_every_action_that_named_it_and_keeps_names_apart() {
        let mut config = Config::default();
        config.general.macros.push(Macro { name: "Macro 1".into(), steps: Vec::new() });
        config.general.macros.push(Macro { name: "Reload".into(), steps: Vec::new() });
        let button = crate::config::Button::South;
        config.general.profiles[0].set_button(button, ButtonAction::Macro { name: "Macro 1".into(), repeat: false });
        assert!(rename(&mut config, ItemKind::Macro, 0, "  Jump  "));
        assert_eq!(config.general.profiles[0].button(button), &ButtonAction::Macro { name: "Jump".into(), repeat: false });
        assert!(rename(&mut config, ItemKind::Macro, 0, "Reload"));
        assert_eq!(config.general.macros[0].name, "Reload (2)");
        assert!(!rename(&mut config, ItemKind::Macro, 0, "   "), "an empty name is refused");
        assert!(in_use(&config, ItemKind::Macro, 0));
        assert!(!in_use(&config, ItemKind::Macro, 1));
    }
}
