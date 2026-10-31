//! The system check: what padwight needs from the system (the checks `padwight doctor` makes),
//! shown as a banner on the Overview, a dialog that says what each thing is for and how to fix it,
//! and a card on the Settings page.

use iced::widget::{column, row};

use super::*;

const OK_COLOR: Color = Color::from_rgb(0.3, 0.7, 0.4);

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
        match *message {
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
                if let Some(fix) = self.system_checks.get(i).and_then(|c| c.fix.clone()) {
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
            column![text(headline), text(first.title.clone()).size(13).color(MUTED_COLOR)].spacing(2).width(Length::Fill),
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
            text("Nothing to check on this system.").color(MUTED_COLOR)
        } else if n == 0 {
            text("✓ Everything Padwight needs is in place.").color(OK_COLOR)
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
            .color(MUTED_COLOR),
        ]
        .spacing(12);
        // Problems first, then notes, then what's fine.
        for severity in [Severity::Problem, Severity::Note, Severity::Ok] {
            for (i, check) in self.system_checks.iter().enumerate().filter(|(_, c)| c.severity == severity) {
                body = body.push(view_check(i, check));
            }
        }
        if self.system_checks.is_empty() {
            body = body.push(text("Nothing to check on this system.").color(MUTED_COLOR));
        }
        body = body
            .push(
                rich_text![
                    "The same report, with what each check is for: ",
                    text::Span::<'static, ()>::new("padwight doctor --explain").font(iced::Font::MONOSPACE),
                    " in a terminal."
                ]
                    .size(12)
                    .color(MUTED_COLOR),
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
    let (mark, color) = match check.severity {
        Severity::Ok => ("✓", OK_COLOR),
        Severity::Note => ("•", WARNING_COLOR),
        Severity::Problem => ("✗", ERROR_COLOR),
    };
    let head = row![text(mark).color(color).width(18), text(check.title.clone()).width(Length::Fill)].spacing(6);
    if check.severity == Severity::Ok {
        return head.into();
    }
    let mut lines = column![].spacing(4);
    for line in check.detail.lines().filter(|l| !l.is_empty()) {
        lines = lines.push(text(line.to_owned()).size(13).color(MUTED_COLOR));
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
