//! The offer, under an input that opens a button or directional menu, to also bind each of the
//! menu's items to a combo of that input and the item's button (RB opens the menu, so RB + D-pad
//! Up does what the menu's Up item does).

use super::*;

/// Where an input opens a menu: the input's buttons (several for a combo), and the menu's name.
/// Only presses and long presses count; a double or triple tap isn't offered.
pub(super) fn menu_combos(trigger: &[Button], action: &ButtonAction, menus: &[&Menu]) -> Vec<Combo> {
    let ButtonAction::OpenMenu(name) = action else { return Vec::new() };
    let Some(menu) = menus.iter().find(|m| &m.name == name) else { return Vec::new() };
    // The buttons that pick each item, in item order. Other menus are moved through, not picked from.
    let picks: Vec<Option<Button>> = match menu.kind {
        MenuKind::Directional { cluster } => cluster.slots().into_iter().map(Some).collect(),
        MenuKind::Buttons => menu.items.iter().map(|item| item.button).collect(),
        _ => return Vec::new(),
    };
    menu.items
        .iter()
        .zip(picks)
        .filter(|(item, _)| item.action != ButtonAction::Disabled)
        .filter_map(|(item, pick)| {
            let pick = pick.filter(|b| !trigger.contains(b))?;
            let buttons = trigger.iter().copied().chain([pick]).collect();
            Some(Combo { buttons, action: item.action.clone() })
        })
        .collect()
}

/// The offer under an input's action editor: a button adding the combos, off (with the reason)
/// once any of them clashes with a combo already there. `None` when the action isn't a menu
/// that has any.
pub(super) fn offer<'a>(p: &Profile, trigger: &[Button], action: &ButtonAction, ui: &Ui) -> Option<Element<'a, Message>> {
    let combos = menu_combos(trigger, action, &ui.menus);
    if combos.is_empty() {
        return None;
    }
    let taken = combos.iter().any(|c| p.combos.iter().any(|e| crate::config::combo_key(&e.buttons) == crate::config::combo_key(&c.buttons)));
    let names = combos.iter().map(|c| c.buttons.iter().map(|b| short_button(*b)).collect::<Vec<_>>().join(" + ")).collect::<Vec<_>>().join(", ");
    let add = button(text("+ Add menu combos").size(13)).style(style::secondary).padding([3, 10]).on_press_maybe((!taken).then(|| Message::AddMenuCombos(combos)));
    let hint = if taken { "Some of these combos already exist.".to_string() } else { format!("Adds {names}.") };
    let tip = help(
        "Makes a combo per menu item: this input plus the item's button (a D-pad direction, face button or quick-select button) does what the item does. \
         They're ordinary combos on the Combos tab, so you can edit or delete them. Unavailable once any of them exists."
            .into(),
    );
    Some(row![add, tip, note(hint).size(12)].spacing(10).align_y(Alignment::Center).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu(kind: MenuKind, items: Vec<MenuItem>) -> Menu {
        Menu { name: "M".into(), kind, items, cancel: None, no_cancel: false, keep_open: false, start: 0, style: OverlayStyle::default() }
    }

    fn open() -> ButtonAction {
        ButtonAction::OpenMenu("M".into())
    }

    #[test]
    fn directional_menu_gives_a_combo_per_enabled_slot() {
        let items = vec![
            MenuItem::new("Up", ButtonAction::Screenshot),
            MenuItem::new("Right", ButtonAction::Disabled),
            MenuItem::new("Down", ButtonAction::ToggleMedia),
        ];
        let m = menu(MenuKind::Directional { cluster: Cluster::DPad }, items);
        let combos = menu_combos(&[Button::RightBumper], &open(), &[&m]);
        assert_eq!(combos.len(), 2);
        assert_eq!(combos[0].buttons, [Button::RightBumper, Button::DpadUp]);
        assert_eq!(combos[1].buttons, [Button::RightBumper, Button::DpadDown]);
        assert_eq!(combos[1].action, ButtonAction::ToggleMedia);
    }

    #[test]
    fn a_combo_trigger_grows_and_a_shared_button_is_skipped() {
        let items = vec![MenuItem::new("N", ButtonAction::Screenshot), MenuItem::new("E", ButtonAction::Screenshot)];
        let m = menu(MenuKind::Directional { cluster: Cluster::FaceButtons }, items);
        let combos = menu_combos(&[Button::LeftBumper, Button::North], &open(), &[&m]);
        assert_eq!(combos.len(), 1);
        assert_eq!(combos[0].buttons, [Button::LeftBumper, Button::North, Button::East]);
    }

    #[test]
    fn button_menus_use_item_buttons_and_other_menus_none() {
        let mut item = MenuItem::new("A", ButtonAction::Screenshot);
        item.button = Some(Button::West);
        let m = menu(MenuKind::Buttons, vec![item, MenuItem::new("No button", ButtonAction::Screenshot)]);
        assert_eq!(menu_combos(&[Button::LeftBumper], &open(), &[&m]).len(), 1);
        let list = menu(MenuKind::List, vec![MenuItem::new("A", ButtonAction::Screenshot)]);
        assert!(menu_combos(&[Button::LeftBumper], &open(), &[&list]).is_empty());
        assert!(menu_combos(&[Button::LeftBumper], &ButtonAction::Screenshot, &[&m]).is_empty());
    }
}
