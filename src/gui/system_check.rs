//! The system check: what padwight needs from the system (the checks `padwight doctor` makes),
//! shown as a banner on the Overview, a dialog that says what each thing is for and how to fix it,
//! and a card on the Settings page.

use iced::widget::{column, row};

use super::*;


#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Severity {
    Ok,
    /// Worth knowing; nothing stops working.
    Note,
    /// Something padwight needs doesn't work.
    Problem,
}

pub(super) struct Check {
    pub severity: Severity,
    pub title: String,
    /// What it is for, one paragraph per line.
    pub detail: String,
    /// Shell commands that fix it.
    pub fix: Option<String>,
}

/// Looks at the system. Nothing here needs the daemon.
#[cfg(target_os = "linux")]
pub(super) fn run() -> Vec<Check> {
    use crate::doctor::Level;
    crate::doctor::check_all()
        .into_iter()
        .map(|f| Check {
            severity: match f.level {
                Level::Ok => Severity::Ok,
                Level::Note => Severity::Note,
                Level::Problem => Severity::Problem,
            },
            title: f.title,
            detail: f.detail.replace('`', ""),
            fix: f.fix,
        })
        .collect()
}

/// Made-up findings, for pictures of what a machine with problems shows.
pub(super) fn demo() -> Vec<Check> {
    let check = |severity, title: &str, detail: &str, fix: Option<&str>| Check {
        severity,
        title: title.into(),
        detail: detail.into(),
        fix: fix.map(Into::into),
    };
    vec![
        check(
            Severity::Problem,
            "Virtual devices: no write access to /dev/uinput",
            "Your user can't write to /dev/uinput. 70-padwight-uinput.rules tags it for systemd-logind's `uaccess`, which gives access only to the user at the machine.\npadwight shows games a virtual gamepad, and sends keys and clicks through a virtual keyboard and mouse. The kernel creates virtual input devices only through /dev/uinput, so the daemon must open it for writing.",
            Some("printf '%s\\n' 'KERNEL==\"uinput\", SUBSYSTEM==\"misc\", OPTIONS+=\"static_node=uinput\", TAG+=\"uaccess\"' | sudo tee /etc/udev/rules.d/70-padwight-uinput.rules > /dev/null\nsudo udevadm control --reload && sudo udevadm trigger"),
        ),
        check(
            Severity::Note,
            "Gyro: 70-padwight-motion.rules isn't installed",
            "You need it only for gyro on PlayStation and Switch controllers. Without it, their motion sensors are root-only.",
            Some("printf '%s\\n' 'SUBSYSTEM==\"input\", KERNEL==\"event*\", ENV{ID_INPUT_ACCELEROMETER}==\"1\", DRIVERS==\"playstation|sony|nintendo\", TAG+=\"uaccess\"' | sudo tee /etc/udev/rules.d/70-padwight-motion.rules > /dev/null\nsudo udevadm control --reload && sudo udevadm trigger"),
        ),
        check(Severity::Ok, "Controller: Wireless Controller (event12) is readable", "", None),
        check(Severity::Ok, "Desktop session: found", "", None),
    ]
}

impl App {
    pub(super) fn update_system_check(&mut self, message: &Message) -> Task<Message> {
        match message {
            Message::CopySystemSetup => {
                let script = crate::doctor::setup_script().replace('\'', "'\\''");
                return iced::clipboard::write(format!("sudo /bin/sh -c '{script}'"));
            }
            Message::ApplySystemSetup if !self.setup_running => {
                self.setup_running = true;
                return Task::perform(async {
                    tokio::task::spawn_blocking(crate::doctor::apply_setup)
                        .await.map_err(|e| e.to_string())?
                }, Message::SystemSetupDone);
            }
            Message::SystemSetupDone(result) => {
                self.setup_running = false;
                self.system_checks = run();
                self.message = Some(match result {
                    Ok(()) => ("Setup installed. Reconnect your controller. If access is still missing, log out and back in, then open Padwight again.".into(), false),
                    Err(error) => (error.clone(), true),
                });
                if result.is_ok() && std::env::var_os("PADWIGHT_NO_DAEMON").is_none() {
                    return Task::perform(async {
                        tokio::task::spawn_blocking(crate::daemon::ensure_running)
                            .await.map_err(|e| e.to_string())?
                            .map_err(|e| format!("Setup installed, but the background service could not start: {e:#}"))
                    }, Message::Done);
                }
            }
            Message::OpenSystemCheck => {
                self.system_checks = run();
                self.dialog = Some(Dialog::SystemCheck);
            }
            Message::RecheckSystem => {
                self.system_checks = run();
                let n = self.problem_count();
                self.message = Some((
                    if n == 0 { "Everything Padwight needs is in place.".into() } else { format!("{n} still to fix.") },
                    n > 0,
                ));
            }
            Message::CopyFix(i) => {
                if let Some(fix) = self.system_checks.get(*i).and_then(|c| c.fix.clone()) {
                    self.message = Some(("Copied. Paste it into a terminal; sudo asks for your password.".into(), false));
                    return iced::clipboard::write(fix);
                }
            }
            _ => {}
        }
        Task::none()
    }

    pub(super) fn problem_count(&self) -> usize {
        self.system_checks.iter().filter(|c| c.severity == Severity::Problem).count()
    }

    /// A strip for the Overview when something padwight needs is missing.
    pub(super) fn view_system_check_banner(&self) -> Option<Element<'_, Message>> {
        let first = self.system_checks.iter().find(|c| c.severity == Severity::Problem)?;
        let n = self.problem_count();
        let headline = if n == 1 { "One thing Padwight needs isn't set up".to_owned() } else { format!("{n} things Padwight needs aren't set up") };
        let strip = row![
            text("✗").size(18).color(ERROR_COLOR),
            column![text(headline), text(first.title.clone()).size(13).style(muted)].spacing(2).width(Length::Fill),
            button(text("Review…")).style(style::primary).on_press(Message::OpenSystemCheck),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        Some(container(strip).padding(12).width(Length::Fill).style(style::card).into())
    }

    /// The Help card's system check line: the way into the dialog, and a summary.
    pub(super) fn system_check_row(&self) -> Element<'_, Message> {
        let n = self.problem_count();
        let summary = if self.system_checks.is_empty() {
            text("Nothing to check on this system.").style(muted)
        } else if n == 0 {
            text("✓ Everything Padwight needs is in place.").style(ok_text)
        } else {
            text(format!("✗ {n} to fix.")).color(ERROR_COLOR)
        };
        row![button(text("System check…")).style(style::secondary).on_press(Message::OpenSystemCheck), summary]
            .spacing(12)
            .align_y(Alignment::Center)
            .into()
    }

    pub(super) fn view_system_check(&self) -> Element<'_, Message> {
        let mut body = column![
            text("System check").size(22),
            text(
                "Padwight needs a few things from your system. Each line says why, and for anything missing, the \
                 commands that set it up. ✗ doesn't work yet · • worth knowing · ✓ in place."
            )
            .size(13)
            .style(muted),
        ]
        .spacing(12);
        body = body.push(text("System setup preview").size(18))
            .push(text("Apply setup writes or replaces the four files below (mode 0644). The uinput rule lets the active local user create virtual input devices; the motion rule permits PlayStation and Nintendo gyro access. The hidraw rule prevents duplicate input in Steam and games, but also disables their gyro and light controls while Padwight is off. The module file loads uinput at boot. The final commands load it now and reload and apply the rules. Reconnect your controller afterwards. Administrator authorization is required.").size(13))
            .push(container(scrollable(text(crate::doctor::setup_script()).size(12).font(iced::Font::MONOSPACE)).height(200)).padding(8).style(style::inset))
            .push(row![button(text("Copy setup commands")).style(style::secondary).on_press(Message::CopySystemSetup), {
                let apply = button(text(if self.setup_running { "Applying setup…" } else { "Apply setup…" })).style(style::primary);
                if self.setup_running { apply } else { apply.on_press(Message::ApplySystemSetup) }
            }].spacing(8));
        // Problems first, then notes, then what's fine.
        for severity in [Severity::Problem, Severity::Note, Severity::Ok] {
            for (i, check) in self.system_checks.iter().enumerate().filter(|(_, c)| c.severity == severity) {
                body = body.push(view_check(i, check));
            }
        }
        if self.system_checks.is_empty() {
            body = body.push(text("Nothing to check on this system.").style(muted));
        }
        body = body
            .push(
                rich_text![
                    "The same report, with what each check is for: ",
                    text::Span::<'static, ()>::new("padwight doctor --explain").font(iced::Font::MONOSPACE),
                    " in a terminal."
                ]
                    .size(12)
                    .style(muted),
            )
            .push(
                row![
                    space::horizontal(),
                    button(text("Check again")).style(style::secondary).on_press(Message::RecheckSystem),
                    button(text("Close")).style(style::primary).on_press(Message::CloseDialog),
                ]
                .spacing(8),
            );
        container(body).width(620).into()
    }
}

fn view_check(i: usize, check: &Check) -> Element<'_, Message> {
    let (mark, style): (_, fn(&iced::Theme) -> iced::widget::text::Style) = match check.severity {
        Severity::Ok => ("✓", ok_text),
        Severity::Note => ("•", warning_text),
        Severity::Problem => ("✗", |_| iced::widget::text::Style { color: Some(ERROR_COLOR) }),
    };
    let head = row![text(mark).style(style).width(18), text(check.title.clone()).width(Length::Fill)].spacing(6);
    if check.severity == Severity::Ok {
        return head.into();
    }
    let mut lines = column![].spacing(4);
    for line in check.detail.lines().filter(|l| !l.is_empty()) {
        lines = lines.push(text(line.to_owned()).size(13).style(muted));
    }
    let mut card = column![head, lines].spacing(6);
    if let Some(fix) = &check.fix {
        card = card.push(text("To fix it, run this in a terminal:").size(13));
        card = card.push(
            container(
                row![
                    text(fix.clone()).size(12).font(iced::Font::MONOSPACE).width(Length::Fill),
                    button(text("Copy").size(13)).style(style::secondary).on_press(Message::CopyFix(i)),
                ]
                .spacing(8)
                .align_y(Alignment::Start),
            )
            .padding(8)
            .width(Length::Fill)
            .style(style::inset),
        );
    }
    container(card).padding(12).width(Length::Fill).style(style::card).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelled_setup_keeps_the_preview_open_and_allows_retry() {
        let mut app = super::super::tests::app();
        let _ = app.update(Message::OpenSystemCheck);
        app.setup_running = true;
        let _ = app.update(Message::SystemSetupDone(Err("Authorization cancelled".into())));
        assert!(!app.setup_running);
        assert!(matches!(app.dialog, Some(Dialog::SystemCheck)));
        assert_eq!(app.message, Some(("Authorization cancelled".into(), true)));
        // Rendering the preview after cancellation must remain possible.
        let _ = app.view_system_check();
    }
}
