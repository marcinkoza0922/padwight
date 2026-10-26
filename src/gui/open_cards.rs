//! Which cards are open is kept by position in their list. These move it along when the list
//! changes, so the same items stay open.

use super::*;

/// What the open cards refer to: the shown game's item names and the edited profile's combos.
/// Taken before the config is replaced wholesale (Undo, Revert), to find each item again after.
pub(super) struct Shown {
    items: [Vec<String>; 5],
    combos: Vec<Vec<Button>>,
}

/// The lists whose cards are kept by position, in `Shown::items` order.
const LISTS: [ItemKind; 5] = [ItemKind::Macro, ItemKind::Cycle, ItemKind::Menu, ItemKind::Info, ItemKind::Log];

/// Where an item went after item `i` was removed: later items move up one place.
pub(super) fn after_removing(i: usize) -> impl Fn(usize) -> Option<usize> {
    move |j| match j.cmp(&i) {
        std::cmp::Ordering::Less => Some(j),
        std::cmp::Ordering::Equal => None,
        std::cmp::Ordering::Greater => Some(j - 1),
    }
}

/// Where an item went after `n` were put in front of it.
pub(super) fn after_inserting(n: usize) -> impl Fn(usize) -> Option<usize> {
    move |j| Some(j + n)
}

/// Where item `i` of `old` is in `new`: the same one by key, else the same place if the list
/// kept its length (an edit to the key itself was undone).
fn find_again<K: PartialEq>(old: &[K], new: &[K], i: usize) -> Option<usize> {
    let key = old.get(i)?;
    new.iter().position(|k| k == key).or_else(|| (old.len() == new.len()).then_some(i))
}

fn moved(set: &HashSet<usize>, to: &impl Fn(usize) -> Option<usize>) -> HashSet<usize> {
    set.iter().filter_map(|j| to(*j)).collect()
}

impl App {
    fn item_names(&self, kind: ItemKind) -> Vec<String> {
        let game = self.game();
        match kind {
            ItemKind::Macro => game.macros.iter().map(|m| m.name.clone()).collect(),
            ItemKind::Cycle => game.cycles.iter().map(|c| c.name.clone()).collect(),
            ItemKind::Menu => game.menus.iter().map(|m| m.name.clone()).collect(),
            ItemKind::Info => game.info.iter().map(|o| o.name.clone()).collect(),
            ItemKind::Log => game.logs.iter().map(|o| o.name.clone()).collect(),
            ItemKind::Layer | ItemKind::Mode => Vec::new(),
        }
    }

    pub(super) fn shown(&self) -> Shown {
        Shown {
            items: LISTS.map(|kind| self.item_names(kind)),
            combos: self.profile().map_or_else(Vec::new, |p| p.combos.iter().map(|c| c.buttons.clone()).collect()),
        }
    }

    /// Moves the open cards of `kind`'s items to where `to` says each item went (`None`: gone).
    pub(super) fn move_open_cards(&mut self, kind: ItemKind, to: impl Fn(usize) -> Option<usize>) {
        match kind {
            ItemKind::Macro => self.open_macros = moved(&self.open_macros, &to),
            ItemKind::Cycle => self.open_cycles = moved(&self.open_cycles, &to),
            ItemKind::Menu => {
                self.open_menus = moved(&self.open_menus, &to);
                // `None` is the keyboard's, which isn't in the list.
                self.open_appearance = self.open_appearance.iter().filter_map(|a| a.map_or(Some(None), |j| to(j).map(Some))).collect();
            }
            ItemKind::Info => {
                self.open_infos = moved(&self.open_infos, &to);
                self.open_info_appearance = moved(&self.open_info_appearance, &to);
            }
            ItemKind::Log => {
                self.open_logs = moved(&self.open_logs, &to);
                self.open_log_appearance = moved(&self.open_log_appearance, &to);
            }
            ItemKind::Layer | ItemKind::Mode => return,
        }
        self.open_input_groups = self
            .open_input_groups
            .iter()
            .filter_map(|&(k, i, g)| if k == kind { to(i).map(|i| (k, i, g)) } else { Some((k, i, g)) })
            .collect();
    }

    /// Moves the open combo rows to where `to` says each combo went.
    pub(super) fn move_open_combos(&mut self, to: impl Fn(usize) -> Option<usize>) {
        self.expanded = self
            .expanded
            .iter()
            .filter_map(|t| match *t {
                Target::Combo(i) => to(i).map(Target::Combo),
                other => Some(other),
            })
            .collect();
    }

    /// After the config was replaced wholesale, keeps the cards open on the items they were open on.
    pub(super) fn follow_open_cards(&mut self, before: &Shown) {
        for (kind, old) in LISTS.into_iter().zip(&before.items) {
            let new = self.item_names(kind);
            self.move_open_cards(kind, |i| find_again(old, &new, i));
        }
        self.refresh_layer_view();
        let combos = self.shown().combos;
        self.move_open_combos(|i| find_again(&before.combos, &combos, i));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::tests::*;

    /// Two macros, "Macro 2" above "Macro", with only the lower one's card open.
    fn two_macros() -> App {
        let mut app = app();
        let _ = app.update(Message::NewMacro);
        let _ = app.update(Message::NewMacro);
        app.open_macros = HashSet::from([1]);
        app.last_edit = None;
        app
    }

    #[test]
    fn undo_keeps_the_same_cards_open() {
        let mut app = two_macros();
        let _ = app.update(Message::NewMacro);
        assert_eq!(app.open_macros, HashSet::from([0, 2]));
        let _ = app.update(Message::Undo);
        assert_eq!(app.macros().iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["Macro 2", "Macro"]);
        assert_eq!(app.open_macros, HashSet::from([1]), "\"Macro\" is still the open one");

        // Undoing a delete opens nothing new: the card that was open stays with its item.
        let _ = app.update(Message::DeleteMacro(0));
        assert_eq!(app.open_macros, HashSet::from([0]));
        let _ = app.update(Message::Undo);
        assert_eq!(app.open_macros, HashSet::from([1]));

        // Undoing a rename keeps the card open on the item, under its old name.
        let _ = app.update(Message::RenameMacro(1, "Jump spam".into()));
        let _ = app.update(Message::Undo);
        assert_eq!(app.open_macros, HashSet::from([1]));
    }

    #[test]
    fn revert_keeps_the_same_cards_open() {
        let mut app = two_macros();
        app.saved = app.config.clone();
        let _ = app.update(Message::NewMacro);
        let _ = app.update(Message::Revert);
        assert_eq!(app.open_macros, HashSet::from([1]));
    }

    #[test]
    fn input_groups_move_with_their_overlay() {
        let mut app = app();
        let _ = app.update(Message::NewInfo);
        let _ = app.update(Message::ToggleInputGroup(ItemKind::Info, 0, 2));
        let _ = app.update(Message::NewLog);
        let _ = app.update(Message::ToggleInputGroup(ItemKind::Log, 0, 1));
        let _ = app.update(Message::NewInfo);
        assert_eq!(app.open_input_groups, HashSet::from([(ItemKind::Info, 1, 2), (ItemKind::Log, 0, 1)]));
        let _ = app.update(Message::DeleteInfo(1));
        assert_eq!(app.open_input_groups, HashSet::from([(ItemKind::Log, 0, 1)]), "a deleted overlay's groups go with it");
    }

    #[test]
    fn open_combos_follow_removal_and_undo() {
        let mut app = app();
        let _ = app.update(Message::AddCombo);
        let _ = app.update(Message::AddCombo);
        let _ = app.update(Message::AddComboButton(1, Button::South));
        app.expanded = HashSet::from([Target::Combo(1)]);
        app.last_edit = None;
        let _ = app.update(Message::RemoveCombo(0));
        assert_eq!(app.expanded, HashSet::from([Target::Combo(0)]));
        let _ = app.update(Message::Undo);
        assert_eq!(app.expanded, HashSet::from([Target::Combo(1)]));
    }

    #[test]
    fn a_moved_card_is_found_by_its_key_or_kept_in_place() {
        let (old, new) = (["a", "b", "c"], ["c", "a"]);
        assert_eq!(find_again(&old, &new, 0), Some(1));
        assert_eq!(find_again(&old, &new, 1), None, "gone, and the list changed length");
        assert_eq!(find_again(&old, &["a", "x", "c"], 1), Some(1), "renamed in place");
        assert_eq!(find_again(&old, &new, 5), None);
    }
}
