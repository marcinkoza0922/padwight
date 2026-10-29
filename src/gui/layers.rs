//! The Modes and Layers tabs: a game's modes and layers, edited with the profile editor over one
//! of its profiles. A mode and a layer are built the same way; only how they're turned on
//! differs, so both tabs are this one editor.
//!
//! The editor works on `App::layer_view`, the compared profile with the mode or layer on top
//! (and only its own combos). After each message, whatever changed there is written back into
//! it as an override (see [`App::write_back`]).

use iced::widget::{column, row};

use super::*;
use crate::config::{Indicator, Layer};

mod shift;

pub(super) use shift::Shift;

/// An entry in a layer's "Indicator" picker.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct IndicatorChoice(pub Indicator);

impl fmt::Display for IndicatorChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Indicator::Name => f.write_str("Its name"),
            Indicator::Info(name) => write!(f, "Info overlay “{name}”"),
            Indicator::Bindings => f.write_str("Its bindings (generated)"),
            Indicator::Off => f.write_str("Nothing"),
        }
    }
}

impl App {
    /// Which of the Modes and Layers tabs is shown, if one is.
    pub(super) fn shift(&self) -> Option<Shift> {
        match (&self.page, self.game_tab) {
            (Page::Game(_), GameTab::Modes) => Some(Shift::Mode),
            (Page::Game(_), GameTab::Layers) => Some(Shift::Layer),
            _ => None,
        }
    }

    fn shifts(&self, shift: Shift) -> &Vec<Layer> {
        match shift {
            Shift::Mode => &self.game().modes,
            Shift::Layer => &self.game().layers,
        }
    }

    fn shifts_mut(&mut self, shift: Shift) -> &mut Vec<Layer> {
        match shift {
            Shift::Mode => &mut self.game_mut().modes,
            Shift::Layer => &mut self.game_mut().layers,
        }
    }

    /// Which mode or layer the tab shows, by index.
    fn shift_index(&self, shift: Shift) -> usize {
        match shift {
            Shift::Mode => self.mode,
            Shift::Layer => self.layer,
        }
    }

    fn set_shift_index(&mut self, shift: Shift, i: usize) {
        match shift {
            Shift::Mode => self.mode = i,
            Shift::Layer => self.layer = i,
        }
    }

    /// The mode or layer the shown tab edits.
    pub(super) fn edited(&self) -> Option<&Layer> {
        let shift = self.shift()?;
        self.shifts(shift).get(self.shift_index(shift))
    }

    pub(super) fn edited_mut(&mut self) -> Option<&mut Layer> {
        let shift = self.shift()?;
        let i = self.shift_index(shift);
        self.shifts_mut(shift).get_mut(i)
    }

    /// The Modes or Layers tab shows a mode or layer, so the profile editor edits it.
    pub(super) fn editing_layer(&self) -> bool {
        self.edited().is_some()
    }

    /// The profile a layer is compared with (and shown over).
    fn compared(&self) -> Option<&Profile> {
        let profiles = &self.game().profiles;
        profiles.get(self.compare).or(profiles.first())
    }

    /// Rebuilds the profile the layer editor shows.
    pub(super) fn refresh_layer_view(&mut self) {
        self.layer_view = if let Some(layer) = self.edited() {
            self.compared().map(|base| {
                let mut view = base.with_layers([layer]);
                view.combos = layer.combos.clone();
                view
            })
        } else {
            None
        };
    }

    /// Records what an edit changed in the layer editor as the layer's overrides.
    pub(super) fn write_back(&mut self, before: &Profile) {
        let Some(after) = self.layer_view.clone() else { return };
        let Some(layer) = self.edited_mut() else { return };
        for b in Button::EVERY.into_iter().chain(Button::PADDLES) {
            if before.buttons.get(&b) != after.buttons.get(&b) {
                layer.buttons.insert(b, after.button(b).clone());
            }
            if before.gestures.get(&b) != after.gestures.get(&b) {
                // Empty gestures in a layer mean "none while it's on".
                layer.gestures.insert(b, after.gestures.get(&b).cloned().unwrap_or_default());
            }
        }
        if before.left_stick != after.left_stick {
            layer.left_stick = Some(after.left_stick.clone());
        }
        if before.right_stick != after.right_stick {
            layer.right_stick = Some(after.right_stick.clone());
        }
        if before.left_trigger != after.left_trigger {
            layer.left_trigger = Some(after.left_trigger.clone());
        }
        if before.right_trigger != after.right_trigger {
            layer.right_trigger = Some(after.right_trigger.clone());
        }
        if before.gyro != after.gyro {
            layer.gyro = Some(after.gyro.clone());
        }
        if before.touchpad != after.touchpad {
            layer.touchpad = Some(after.touchpad.clone());
        }
        if before.combos != after.combos {
            layer.combos = after.combos.clone();
        }
    }

    /// Makes a new mode or layer in the shown game, named "Mode", "Mode 2", … (or "Layer", …),
    /// and returns its name.
    pub(super) fn add_layer(&mut self, shift: Shift) -> String {
        let name = unique_name(shift.title(), |n| self.shifts(shift).iter().any(|l| l.name == n));
        self.shifts_mut(shift).push(Layer::new(&name));
        name
    }

    /// "+ New layer" picked in a "Hold layer…" action, or "+ New mode" in a "Switch mode…": makes
    /// it and names it there.
    pub(super) fn new_layer_for(&mut self, message: Message) -> Message {
        let Message::SetAction(target, mut action) = message else { return message };
        for shift in [Shift::Layer, Shift::Mode] {
            let wanted = |a: &ButtonAction| match (shift, a) {
                (Shift::Layer, ButtonAction::Layer(n)) => n == NEW_LAYER,
                (Shift::Mode, ButtonAction::Mode(n)) => n == NEW_MODE,
                _ => false,
            };
            let mut wants = false;
            action.walk(&mut |a| wants |= wanted(a));
            if !wants {
                continue;
            }
            let name = self.add_layer(shift);
            action.walk_mut(&mut |a| {
                if wanted(a)
                    && let ButtonAction::Layer(n) | ButtonAction::Mode(n) = a
                {
                    *n = name.clone();
                }
            });
        }
        Message::SetAction(target, action)
    }

    /// The settings of the mode or layer being edited that are a single value or list.
    fn set_layer_option(&mut self, message: Message) {
        let Some(l) = self.edited_mut() else { return };
        match message {
            Message::SetIndicatorTitle(title) => l.indicator_title = Some(title).filter(|t| !t.is_empty()),
            Message::SetIndicatorDelay(ms) => l.indicator_delay_ms = ms.min(10_000) as u32,
            Message::SetSwallowUnbound(on) => l.swallow_unbound = on,
            _ => {}
        }
    }

    #[expect(clippy::too_many_lines, reason = "predates the size lints")]
    pub(super) fn update_layers(&mut self, message: Message) -> Task<Message> {
        let shift = self.shift().unwrap_or(Shift::Layer);
        let i = self.shift_index(shift);
        match message {
            Message::NewLayer => {
                let name = self.add_layer(shift);
                self.set_shift_index(shift, self.shifts(shift).len() - 1);
                let how = match shift {
                    Shift::Mode => "Switch to it with \"Switch mode…\" on any input or menu item.",
                    Shift::Layer => "Hold it with \"Hold layer…\" on any input.",
                };
                self.message = Some((format!("Added {} “{name}”. {how}", shift.kind().noun()), false));
            }
            Message::SelectLayer(name) => {
                if let Some(i) = self.shifts(shift).iter().position(|l| l.name == name) {
                    self.set_shift_index(shift, i);
                    self.expanded.clear();
                }
            }
            Message::RenameLayer(name) => {
                let taken = self.shifts(shift).iter().enumerate().any(|(j, l)| j != i && l.name == name);
                if let Some(old) = self.shifts(shift).get(i).map(|l| l.name.clone())
                    && !taken
                {
                    let game = self.game_mut();
                    game.rename_item(shift.kind(), &old, &name);
                    game.rename_refs(shift.kind(), &old, &name);
                }
            }
            Message::DeleteLayer => {
                if i < self.shifts(shift).len() {
                    self.shifts_mut(shift).remove(i);
                    self.set_shift_index(shift, i.saturating_sub(1));
                }
            }
            Message::SetCompare(name) => {
                if let Some(i) = self.game().profiles.iter().position(|p| p.name == name) {
                    self.compare = i;
                }
            }
            Message::SetIndicator(choice) => {
                if let Some(l) = self.edited_mut() {
                    l.indicator = choice.0;
                }
            }
            Message::SetIndicatorTitle(_)
            | Message::SetIndicatorDelay(_)
            | Message::SetSwallowUnbound(_) => self.set_layer_option(message),
            Message::SetIndicatorStyle(style) => {
                if let Some(l) = self.edited_mut() {
                    l.indicator_style = style;
                }
            }
            Message::ToggleIndicatorAppearance => self.indicator_appearance = !self.indicator_appearance,
            Message::OverrideInput(part) => {
                let Some(base) = self.compared().cloned() else { return Task::none() };
                let Some(layer) = self.edited_mut() else { return Task::none() };
                match part {
                    LayerPart::Button(b) => {
                        layer.buttons.insert(b, base.button(b).clone());
                        if let Some(g) = base.gestures(b) {
                            layer.gestures.insert(b, g.clone());
                        }
                        self.expanded.insert(Target::Button(b));
                    }
                    LayerPart::Stick(Stick::Left) => layer.left_stick = Some(base.left_stick),
                    LayerPart::Stick(Stick::Right) => layer.right_stick = Some(base.right_stick),
                    LayerPart::Trigger(Trigger::Left) => layer.left_trigger = Some(base.left_trigger),
                    LayerPart::Trigger(Trigger::Right) => layer.right_trigger = Some(base.right_trigger),
                    LayerPart::Gyro => layer.gyro = Some(base.gyro),
                    LayerPart::Touchpad => layer.touchpad = Some(base.touchpad),
                }
            }
            Message::RevertInput(part) => {
                let Some(layer) = self.edited_mut() else { return Task::none() };
                match part {
                    LayerPart::Button(b) => {
                        layer.buttons.remove(&b);
                        layer.gestures.remove(&b);
                    }
                    LayerPart::Stick(Stick::Left) => layer.left_stick = None,
                    LayerPart::Stick(Stick::Right) => layer.right_stick = None,
                    LayerPart::Trigger(Trigger::Left) => layer.left_trigger = None,
                    LayerPart::Trigger(Trigger::Right) => layer.right_trigger = None,
                    LayerPart::Gyro => layer.gyro = None,
                    LayerPart::Touchpad => layer.touchpad = None,
                }
            }
            Message::ToggleBaseCombo(key) => {
                if let Some(layer) = self.edited_mut() {
                    let before = layer.disabled_combos.len();
                    layer.disabled_combos.retain(|d| crate::config::combo_key(d) != key);
                    if layer.disabled_combos.len() == before {
                        layer.disabled_combos.push(key);
                    }
                }
            }
            other => return self.update_packs(other),
        }
        Task::none()
    }

    #[expect(clippy::too_many_lines, reason = "predates the size lints")]
    pub(super) fn view_layers<'a>(&'a self, shift: Shift, names: &Names) -> Element<'a, Message> {
        let game = self.game();
        let words = shift.words();
        let list = self.shifts(shift);
        let picker_row = {
            let mut picker = row![].spacing(8).align_y(Alignment::Center);
            // The picker only shows once there's one to pick.
            if !list.is_empty() {
                let names: Vec<String> = list.iter().map(|l| l.name.clone()).collect();
                let current = list.get(self.shift_index(shift)).map(|l| l.name.clone());
                picker = picker.push(dropdown(names, current, Message::SelectLayer).width(220));
            }
            let add = picker.push(button(text(words.new)).style(style::secondary).on_press(Message::NewLayer));
            items_header(shift.kind(), add.into(), words.blurb, words.explain.into(), list.is_empty().then_some(words.empty))
        };
        let mut col = column![picker_row].spacing(16);
        let Some(layer) = list.get(self.shift_index(shift)) else {
            return col.into();
        };

        // Name, indicator and the profile it's shown over.
        let mut indicators =
            vec![IndicatorChoice(Indicator::Name), IndicatorChoice(Indicator::Bindings), IndicatorChoice(Indicator::Off)];
        indicators.extend(names.infos.iter().map(|n| IndicatorChoice(Indicator::Info(n.clone()))));
        let profiles: Vec<String> = game.profiles.iter().map(|p| p.name.clone()).collect();
        let compared = self.compared().map(|p| p.name.clone());
        let mut settings: Vec<Element<'a, Message>> = vec![
            labeled(
                "Name",
                row![
                    field(words.name, &layer.name).on_input(Message::RenameLayer).width(220),
                    space::horizontal(),
                    delete_item(words.delete, Message::DeleteLayer),
                ]
                .align_y(Alignment::Center)
                .into(),
            ),
            labeled(
                "On screen while on",
                row![
                    dropdown(indicators, Some(IndicatorChoice(layer.indicator.clone())), Message::SetIndicator).width(260),
                    help(words.indicator.into()),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
            ),
        ];
        if layer.indicator == Indicator::Bindings {
            settings.push(labeled(
                "Heading",
                row![
                    field("Optional, e.g. Hold {guide} and press", layer.indicator_title.as_deref().unwrap_or("")).on_input(Message::SetIndicatorTitle).width(320),
                    insert_button(TokenTarget::LayerHeading),
                    char_button(TokenTarget::LayerHeading),
                ]
                .push(token_preview(layer.indicator_title.as_deref().unwrap_or(""), self.config.info_glyphs, self.preview_font()))
                .spacing(6)
                .align_y(Alignment::Center)
                .into(),
            ));
        }
        settings.push(labeled(
            "Show after",
            row![
                ms_field(layer.indicator_delay_ms.into(), Message::SetIndicatorDelay),
                help(words.delay.into()),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        ));
        if let Indicator::Info(name) = &layer.indicator
            && !names.infos.contains(name)
        {
            settings.push(labeled("", text(format!("⚠ missing info overlay {name:?}")).size(12).color(ERROR_COLOR).into()));
        }
        if matches!(layer.indicator, Indicator::Name | Indicator::Bindings) {
            settings.push(labeled("", disclosure("Appearance", self.indicator_appearance, Message::ToggleIndicatorAppearance)));
            if self.indicator_appearance {
                settings.push(text_style_editor(&layer.indicator_style, Rc::new(Message::SetIndicatorStyle)));
            }
            let sample = InfoOverlay {
                name: layer.name.clone(),
                always: true, on_start: None, linger: None, title: None, current_input: Default::default(),
                style: preview_style(&layer.indicator_style),
                rows: vec![vec![layer.name.clone()]],
            };
            let view = crate::info::resolve(&sample, &crate::info::Live::sample(self.config.info_glyphs).with_layout(self.nintendo_layout()));
            settings.push(preview(crate::overlay::draw::info_panel(&view, self.preview_font(), &crate::overlay::fit::Fit::contents(), &crate::motion::Anim::still())));
        }
        settings.push(labeled(
            "Unset buttons",
            checkbox(layer.swallow_unbound)
                .label(words.swallow)
                .size(14)
                .text_size(12)
                .on_toggle(Message::SetSwallowUnbound)
                .into(),
        ));
        settings.push(labeled(
            "Shown over",
            row![
                dropdown(profiles, compared, Message::SetCompare).width(220),
                help(words.over.into()),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        ));
        col = col.push(column(settings).spacing(10));

        if let (Some(p), Some(base)) = (self.profile(), self.compared()) {
            col = col.push(self.view_profile_editor(p, names, Some(LayerMarks { layer, base })));
        }
        col.into()
    }
}

#[cfg(test)]
mod tests;
