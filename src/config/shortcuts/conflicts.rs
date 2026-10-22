//! What's wrong with where the shortcuts sit: two on one input, or a profile that already uses
//! the opener's.

use super::{Button, ButtonAction, Group, GuideOpener, Profile, Shortcut, ShortcutInput, Shortcuts};

impl Shortcuts {
    /// What's wrong with where `s` sits: other shortcuts it can't work beside. Switched off is
    /// no problem, except for the opener (see [`Shortcuts::opener_off`]).
    pub fn warnings(&self, s: Shortcut) -> Vec<String> {
        let Some(mine) = self.input(s) else { return Vec::new() };
        let mut found = Vec::new();
        for other in Shortcut::ALL.into_iter().filter(|o| *o != s) {
            let Some(theirs) = self.input(other) else { continue };
            let clash = match (s.group(), other.group()) {
                // The opener is what holds the Guide mode, and it ends mouse mode, so every
                // shortcut must stay clear of it; a held button is no shortcut either.
                (Group::Opener, _) | (_, Group::Opener) => {
                    let held = matches!(self.engine_opener(), GuideOpener::Button(_));
                    let (op, sh) = if s == Shortcut::OpenGuide { (&mine, &theirs) } else { (&theirs, &mine) };
                    op.overlaps(sh) || (held && sh.buttons.iter().any(|b| op.uses(*b)))
                }
                // Confirm and back are pressed in the Guide overlay, so the Guide shortcuts can't use them.
                (Group::Menu, Group::Guide) | (Group::Guide, Group::Menu) => mine.overlaps(&theirs),
                (a, b) => a == b && mine.overlaps(&theirs),
            };
            if clash {
                found.push(format!("{} is on {}, which clashes with this.", other.label(), theirs.describe()));
            }
        }
        found
    }

    /// Every warning, one line each, for the page's summary.
    pub fn all_warnings(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if self.opener_off() {
            lines.push("Nothing opens the Guide overlay, so its menu, Notes, Mappings, the Guide shortcuts and mouse mode are out of reach.".into());
        }
        for s in Shortcut::ALL {
            lines.extend(self.warnings(s).into_iter().map(|w| format!("{}: {w}", s.label())));
        }
        lines
    }

    /// Why this profile and the opener don't get along, if they don't: it already uses what the
    /// opener takes.
    pub fn conflict(&self, p: &Profile) -> Option<String> {
        let input = self.opener()?;
        let used = |b: Button| *p.button(b) != ButtonAction::Disabled || p.gestures.get(&b).is_some_and(|g| !g.is_empty());
        let combo_with = |buttons: &[Button]| {
            let key = crate::config::combo_key(buttons);
            p.combos.iter().find(|c| {
                let theirs = crate::config::combo_key(&c.buttons);
                key.iter().all(|b| theirs.contains(b)) || theirs.iter().all(|b| key.contains(b))
            })
        };
        match self.engine_opener() {
            GuideOpener::Guide => None,
            GuideOpener::Button(b) => (used(b) || p.combos.iter().any(|c| c.buttons.contains(&b)))
                .then(|| format!("{} is mapped, and the opener takes it.", input.describe())),
            GuideOpener::Triggered(_) if input.is_combo() => combo_with(&input.buttons)
                .map(|c| format!("The combo {} already does something.", ShortcutInput::combo(&c.buttons).describe())),
            GuideOpener::Triggered(_) => {
                let (b, kind) = (input.buttons.first()?, input.gesture?);
                p.gestures.get(b).and_then(|g| g.get(kind)).map(|_| format!("{} already does something.", input.describe()))
            }
        }
    }
}
