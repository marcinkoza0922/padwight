use std::{fs, os::unix::fs::PermissionsExt, path::Path};

use super::*;

/// A made-up /dev, /sys and udev tree under the temp directory.
struct Tree(PathBuf);

impl Tree {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("padwight-doctor-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for dir in ["dev", "sys", "rules", "modules"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        Tree(root)
    }

    fn roots(&self) -> Roots {
        Roots {
            uinput: self.0.join("dev/uinput"),
            input_dev: self.0.join("dev"),
            sysfs_input: self.0.join("sys"),
            rule_dirs: vec![self.0.join("rules")],
            module_dirs: vec![self.0.join("modules")],
        }
    }

    /// An input node: `kind` is "pad" or "motion". `driver` names the driver bound to it.
    fn node(&self, event: &str, kind: &str, driver: &str) {
        let device = self.0.join("sys").join(event).join("device");
        fs::create_dir_all(device.join("capabilities")).unwrap();
        fs::write(device.join("name"), format!("Test {kind}\n")).unwrap();
        let (key, abs, props) = if kind == "pad" { ("1000000000000 0 0 0 0", "3", "0") } else { ("0", "0", "40") };
        fs::write(device.join("capabilities/key"), format!("{key}\n")).unwrap();
        fs::write(device.join("capabilities/abs"), format!("{abs}\n")).unwrap();
        fs::write(device.join("properties"), format!("{props}\n")).unwrap();
        std::os::unix::fs::symlink(format!("../../drivers/{driver}"), self.0.join("sys").join(event).join("driver")).unwrap();
        fs::write(self.0.join("dev").join(event), "").unwrap();
    }

    fn deny(&self, event: &str) {
        fs::set_permissions(self.0.join("dev").join(event), fs::Permissions::from_mode(0o000)).unwrap();
    }

    fn put(&self, dir: &str, name: &str, text: &str) {
        fs::write(self.0.join(dir).join(name), text).unwrap();
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Permission checks mean nothing to root.
fn is_root() -> bool {
    // SAFETY: getuid cannot fail.
    unsafe { libc::getuid() == 0 }
}

fn titled<'a>(findings: &'a [Finding], start: &str) -> &'a Finding {
    findings.iter().find(|f| f.title.starts_with(start)).unwrap_or_else(|| panic!("no finding starting {start:?}"))
}

#[test]
fn a_missing_uinput_asks_for_the_module() {
    let tree = Tree::new("no-uinput");
    let f = check_uinput(&tree.roots());
    assert_eq!(f.level, Level::Problem);
    let fix = f.fix.unwrap();
    assert!(fix.contains("modprobe uinput") && fix.contains("/etc/modules-load.d/padwight-uinput.conf"), "{fix}");
    assert!(!fix.contains("<<"), "fish has no heredocs: {fix}");
}

#[test]
fn a_writable_uinput_is_fine() {
    let tree = Tree::new("uinput-ok");
    tree.put("dev", "uinput", "");
    assert_eq!(check_uinput(&tree.roots()).level, Level::Ok);
}

#[test]
fn a_denied_uinput_offers_the_rule_unless_it_is_already_installed() {
    if is_root() {
        return;
    }
    let tree = Tree::new("uinput-denied");
    tree.put("dev", "uinput", "");
    fs::set_permissions(tree.0.join("dev/uinput"), fs::Permissions::from_mode(0o000)).unwrap();
    let f = check_uinput(&tree.roots());
    assert_eq!(f.level, Level::Problem);
    assert!(f.fix.as_deref().unwrap().contains("tee /etc/udev/rules.d/70-padwight-uinput.rules"));
    tree.put("rules", UINPUT_RULE_NAME, UINPUT_RULE);
    let f = check_uinput(&tree.roots());
    assert_eq!(f.fix.as_deref(), Some(RELOAD));
}

#[test]
fn a_readable_pad_and_no_rule_is_only_a_note() {
    let tree = Tree::new("pad-ok");
    tree.node("event5", "pad", "xpad");
    let findings = check_controllers(&tree.roots());
    assert_eq!(titled(&findings, "Controller").level, Level::Ok);
    let rule = titled(&findings, "Gyro");
    assert_eq!(rule.level, Level::Note);
    assert!(rule.fix.as_deref().unwrap().contains("70-padwight-motion.rules"));
}

#[test]
fn an_unreadable_playstation_motion_sensor_is_a_problem_with_the_rule() {
    if is_root() {
        return;
    }
    let tree = Tree::new("motion-denied");
    tree.node("event7", "motion", "playstation");
    tree.deny("event7");
    let findings = check_controllers(&tree.roots());
    let f = titled(&findings, "Gyro: no read access");
    assert_eq!(f.level, Level::Problem);
    assert!(f.fix.as_deref().unwrap().contains("ID_INPUT_ACCELEROMETER"));
    assert_eq!(titled(&findings, "No controller").level, Level::Note);
}

#[test]
fn a_laptop_tilt_sensor_is_left_alone() {
    let tree = Tree::new("tilt");
    tree.node("event3", "motion", "some_iio_driver");
    let findings = check_controllers(&tree.roots());
    assert!(findings.iter().all(|f| !f.title.contains("event3")), "{}", render(&findings));
}

#[test]
fn an_unreadable_pad_points_at_the_input_group() {
    if is_root() {
        return;
    }
    let tree = Tree::new("pad-denied");
    tree.node("event5", "pad", "xpad");
    tree.deny("event5");
    let findings = check_controllers(&tree.roots());
    assert!(titled(&findings, "Controller").fix.as_deref().unwrap().contains("usermod -aG input"));
}

#[test]
fn the_motion_rule_is_current_outdated_or_missing() {
    let tree = Tree::new("rule-state");
    let state = || file_state(&tree.roots().rule_dirs, MOTION_RULE_NAME, MOTION_RULE);
    assert_eq!(state(), RuleState::Missing);
    tree.put("rules", MOTION_RULE_NAME, "# old\n");
    assert_eq!(state(), RuleState::Outdated);
    tree.put("rules", MOTION_RULE_NAME, MOTION_RULE);
    assert_eq!(state(), RuleState::Current);
}

#[test]
fn the_hidraw_rule_is_a_note_that_names_a_connected_switch_pad() {
    let tree = Tree::new("hidraw-rule");
    tree.node("event5", "pad", "xpad");
    let findings = check_controllers(&tree.roots());
    let f = titled(&findings, "Raw controller access");
    assert_eq!(f.level, Level::Note);
    assert!(f.fix.as_deref().unwrap().contains("72-padwight-hidraw.rules") && !f.detail.contains("is connected"), "{}", f.detail);
    tree.node("event6", "pad", "nintendo");
    let findings = check_controllers(&tree.roots());
    assert!(titled(&findings, "Raw controller access").detail.starts_with("A PlayStation or Switch controller is connected."));
    tree.put("rules", HIDRAW_RULE_NAME, HIDRAW_RULE);
    let findings = check_controllers(&tree.roots());
    assert_eq!(titled(&findings, "Raw controller access").level, Level::Ok);
}

#[test]
fn bitmaps_read_most_significant_word_first() {
    assert!(devices::scan(Path::new("/nonexistent"), Path::new("/nonexistent")).is_empty());
    let tree = Tree::new("bits");
    tree.node("event5", "pad", "xpad");
    let nodes = devices::scan(&tree.roots().sysfs_input, &tree.roots().input_dev);
    assert!(nodes[0].gamepad && !nodes[0].motion);
}

#[test]
fn the_shipped_files_install_what_the_checks_look_for() {
    assert!(MOTION_RULE.contains("DRIVERS==\"playstation|sony|nintendo\""));
    assert!(UINPUT_RULE.contains("KERNEL==\"uinput\""));
    assert!(HIDRAW_RULE.contains("DRIVERS==\"playstation|sony|nintendo\", MODE:=\"0000\", TAG-=\"uaccess\""));
    assert!(MODULES_CONF.lines().any(|l| l == "uinput"));
}

#[test]
fn only_a_wayland_session_counts_as_a_desktop() {
    assert_eq!(desktop_finding(true, true).level, Level::Ok);
    assert_eq!(desktop_finding(true, false).level, Level::Ok);
    let x11 = desktop_finding(false, true);
    assert!(x11.level == Level::Problem && x11.title.contains("X11"), "{}", x11.title);
    assert_eq!(desktop_finding(false, false).level, Level::Note);
}

#[test]
fn a_missing_wl_copy_is_only_a_note() {
    assert_eq!(clipboard_finding(true).level, Level::Ok);
    let missing = clipboard_finding(false);
    assert!(missing.level == Level::Note && missing.detail.contains("wl-clipboard") && missing.fix.is_none());
}

#[test]
fn a_fix_writes_the_rule_with_printf_and_tee() {
    let fix = install_rule(MOTION_RULE_NAME, MOTION_RULE);
    let first = fix.lines().next().unwrap();
    assert!(first.starts_with("printf '%s\\n' '") && first.ends_with("| sudo tee /etc/udev/rules.d/70-padwight-motion.rules > /dev/null"), "{fix}");
    assert!(fix.lines().nth(1) == Some(RELOAD) && !fix.contains('#'), "{fix}");
}
