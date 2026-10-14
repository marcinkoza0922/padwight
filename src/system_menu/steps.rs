//! The in-game menu's steps on its pages: Quick Settings, Switch Profile, Edit Controls, and
//! typing into one of the editor's fields.

use super::*;

impl SystemMenu {
    /// What the keyboard did while typing into a field: Enter sets the text, and holding B to
    /// close it goes back without.
    pub(super) fn typed(&mut self, actions: Vec<OverlayAction>, config: &mut Config) -> Option<Outcome> {
        for action in actions {
            let done = match action {
                OverlayAction::Key(crate::keycode::KeyCode::KEY_ENTER | crate::keycode::KeyCode::KEY_KPENTER, true) => true,
                OverlayAction::Close => false,
                OverlayAction::Key(..) => continue,
            };
            let Screen::Typing(mut editor, typing) = std::mem::replace(&mut self.screen, Screen::Main) else { return None };
            let changed = done && editor.set_text(config, typing.0.typed()) == EditStep::Changed;
            self.screen = Screen::Editor(editor);
            return changed.then_some(Outcome::Changed);
        }
        None
    }

    pub(super) fn quick_step(&mut self, step: Step, config: &mut Config) -> Option<Outcome> {
        let rows = quick::rows(config, &self.saved);
        let armed = std::mem::take(&mut self.remapping_armed);
        match step {
            Step::Up | Step::Down => {
                self.cursor = wrap(self.cursor, step, rows.len());
                None
            }
            // Turning remapping off takes a second press of A.
            Step::Pick if rows.get(self.cursor) == Some(&Row::Remapping) && !armed => {
                self.remapping_armed = true;
                None
            }
            Step::Left | Step::Right if rows.get(self.cursor) == Some(&Row::Remapping) => None,
            Step::Back if self.from_guide => self.leave(config, Exit::Back),
            Step::Back => {
                self.screen = Screen::Main;
                self.cursor = 0;
                None
            }
            Step::Left | Step::Right | Step::Pick => {
                let row = *rows.get(self.cursor)?;
                let dir = match step {
                    Step::Left => -1,
                    _ => 1,
                };
                quick::adjust(row, dir, self.big_steps, config, &self.saved)
            }
        }
    }

    pub(super) fn profiles_step(&mut self, step: Step, config: &Config) -> Option<Outcome> {
        let rows = profiles::rows(config);
        match step {
            Step::Up | Step::Down | Step::Left | Step::Right => self.cursor = list_move(self.cursor, step, rows.len()),
            Step::Pick => match *rows.get(self.cursor)? {
                profiles::Row::New => {
                    self.screen = Screen::NewProfile;
                    self.cursor = 0;
                }
                row => {
                    let target = profiles::target(row, config)?;
                    if target == config.active_ref() {
                        return self.leave(config, Exit::Close);
                    }
                    return Some(Outcome::Switch(target));
                }
            },
            Step::Back => return self.leave(config, Exit::Back),
        }
        None
    }

    pub(super) fn new_profile_step(&mut self, step: Step, config: &mut Config) -> Option<Outcome> {
        let rows = profiles::new_rows(config);
        match step {
            Step::Up | Step::Down | Step::Left | Step::Right => self.cursor = list_move(self.cursor, step, rows.len()),
            Step::Pick => return profiles::create(*rows.get(self.cursor)?, config).map(Outcome::Added),
            Step::Back => {
                self.screen = Screen::Profiles;
                self.cursor = profiles::rows(config).len() - 1;
            }
        }
        None
    }

    pub(super) fn editor_step(&mut self, step: Step, config: &mut Config) -> Option<Outcome> {
        let Screen::Editor(editor) = &mut self.screen else { return None };
        match step {
            Step::Up => editor.move_cursor((0, -1), config),
            Step::Down => editor.move_cursor((0, 1), config),
            Step::Pick => match editor.choose(config) {
                EditStep::Changed => return Some(Outcome::Changed),
                EditStep::Text { title, text } => {
                    let typing = Typing(OverlayController::for_field(&title, &text));
                    let editor = editor.clone();
                    self.screen = Screen::Typing(editor, Box::new(typing));
                }
                EditStep::Stay | EditStep::Leave => {}
            },
            Step::Back => {
                if editor.back() == EditStep::Leave {
                    if self.from_guide {
                        return self.leave(config, Exit::Back);
                    }
                    self.screen = Screen::Main;
                    self.cursor = 1;
                }
            }
            Step::Left | Step::Right if editor.two_columns(config) => {
                editor.move_cursor((if step == Step::Left { -1 } else { 1 }, 0), config);
            }
            Step::Left => return (editor.adjust(-1, config) == EditStep::Changed).then_some(Outcome::Changed),
            Step::Right => return (editor.adjust(1, config) == EditStep::Changed).then_some(Outcome::Changed),
        }
        None
    }
}
