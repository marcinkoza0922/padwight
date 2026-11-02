//! The checks of the two optional udev rules: motion sensors (gyro) and raw HID nodes.

use super::*;
use devices::Node;

/// Reports whether the motion-sensor rule is installed. A missing rule is a problem only when a
/// pad that needs it is connected (that is reported by its node), so here it's a note.
pub(super) fn check_motion_rule(roots: &Roots, nodes: &[Node]) -> Finding {
    match file_state(&roots.rule_dirs, MOTION_RULE_NAME, MOTION_RULE) {
        RuleState::Current => Finding::new(Level::Ok, format!("Gyro: {MOTION_RULE_NAME} is installed"), ""),
        RuleState::Outdated => Finding::new(
            Level::Note,
            format!("Gyro: the installed {MOTION_RULE_NAME} differs from this version's"),
            "Install this version's to be sure gyro access keeps working.",
        )
        .fix(install_rule(MOTION_RULE_NAME, MOTION_RULE))
        .stale(),
        RuleState::Missing => {
            let needed = if nodes.iter().any(Node::is_pad_motion) {
                "A PlayStation or Switch controller is connected."
            } else {
                "You need it only for gyro on PlayStation and Switch controllers."
            };
            Finding::new(
                Level::Note,
                format!("Gyro: {MOTION_RULE_NAME} isn't installed"),
                // The report's last line already points to `--explain`, as the window's dialog does.
                format!("{needed} Without it, their motion sensors are root-only."),
            )
            .fix(install_rule(MOTION_RULE_NAME, MOTION_RULE))
        }
    }
}

/// Reports whether the rule closing PlayStation and Switch controllers' raw HID nodes is installed.
/// Nothing padwight needs depends on it, so a missing rule is a note.
pub(super) fn check_hidraw_rule(roots: &Roots, nodes: &[Node]) -> Finding {
    match file_state(&roots.rule_dirs, HIDRAW_RULE_NAME, HIDRAW_RULE) {
        RuleState::Current => Finding::new(Level::Ok, format!("Raw controller access: {HIDRAW_RULE_NAME} is installed"), ""),
        RuleState::Outdated => Finding::new(
            Level::Note,
            format!("Raw controller access: the installed {HIDRAW_RULE_NAME} differs from this version's"),
            "Install this version's to be sure Steam and games keep reading those controllers through padwight.",
        )
        .fix(install_rule(HIDRAW_RULE_NAME, HIDRAW_RULE))
        .stale(),
        RuleState::Missing => {
            let connected = if nodes.iter().any(Node::is_hidraw_pad) {
                "A PlayStation or Switch controller is connected. "
            } else {
                ""
            };
            Finding::new(
                Level::Note,
                format!("Raw controller access: {HIDRAW_RULE_NAME} isn't installed"),
                format!(
                    "{connected}Steam and SDL games read PlayStation and Switch controllers through their raw HID node, which padwight \
can't take, so they see your presses as well as the virtual gamepad. The rule closes that node. Turn the controller off and \
on after installing it."
                ),
            )
            .fix(install_rule(HIDRAW_RULE_NAME, HIDRAW_RULE))
        }
    }
}
