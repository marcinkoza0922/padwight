//! `padwight doctor`: checks what the daemon needs from the system (access to /dev/uinput, to the
//! controllers and to their motion sensors, and the udev rules that grant it) and says what to
//! change, why, and the exact commands.

mod devices;
#[cfg(test)]
mod tests;

use std::{
    fs,
    io::ErrorKind,
    path::PathBuf,
};

use anyhow::{Result, bail};

use devices::Node;

const MOTION_RULE_NAME: &str = "70-padwight-motion.rules";
const MOTION_RULE: &str = include_str!("../dist/70-padwight-motion.rules");
const UINPUT_RULE_NAME: &str = "70-padwight-uinput.rules";
const UINPUT_RULE: &str = include_str!("../dist/70-padwight-uinput.rules");
const HIDRAW_RULE_NAME: &str = "72-padwight-hidraw.rules";
const HIDRAW_RULE: &str = include_str!("../dist/72-padwight-hidraw.rules");
const MODULES_CONF_NAME: &str = "padwight-uinput.conf";
const MODULES_CONF: &str = include_str!("../dist/padwight-uinput.conf");

/// What the daemon needs and why, for `padwight doctor --explain`.
const REFERENCE: &str = "\
What padwight needs from the system, and why

1. Write access to /dev/uinput.
   padwight reads your real controller and shows games a virtual gamepad, plus a virtual keyboard
   and mouse for the keys and clicks your mappings send. The kernel makes virtual input devices
   only through /dev/uinput, which needs the uinput kernel module loaded.
   Granted by: dist/70-padwight-uinput.rules (systemd-logind's `uaccess` tag gives the user at
   the machine access and nobody else), and dist/padwight-uinput.conf (loads the module at boot).
   Most desktops already do both.

2. Read access to the controller's /dev/input/eventN node.
   That is where its buttons and sticks come from. padwight also takes the node exclusively, so
   games see only the virtual gamepad and not both. systemd's own rules give the logged-in user
   access to joysticks. Without that, `sudo usermod -aG input $USER` and a new login does.

3. Read access to the controller's motion sensor, for gyro (PlayStation and Switch controllers).
   The motion sensor is a separate input node, classed as an accelerometer, which systemd's rules
   leave to root. dist/70-padwight-motion.rules tags it for the logged-in user, only for the
   playstation, sony and nintendo drivers, so a laptop's tilt sensor is untouched.
   Without it, everything but gyro works.

4. Optional: no access to PlayStation and Switch controllers' raw HID node (/dev/hidrawN).
   Steam and SDL games read those controllers there instead of through the input node, so taking
   the input node doesn't stop them: they see every press as well as the virtual gamepad.
   dist/72-padwight-hidraw.rules closes the raw node to every program (padwight doesn't use it),
   so they read the input node, which padwight holds. While padwight is off they then read it
   there too, which costs Steam's own gyro and lightbar control for those controllers.

5. A graphical session, for the settings window, the on-screen keyboard and the overlays.
   The overlays need a Wayland compositor with layer-shell (KDE Plasma, Sway, Hyprland,
   labwc and other wlroots compositors). GNOME and Cinnamon have none: their overlays are shown by
   padwight's shell extension instead (experimental).

6. Optional: the daemon running. `systemctl --user enable --now padwight` starts it at login.

All udev rules go in /etc/udev/rules.d/ and need root once. `padwight doctor` prints the
exact commands for whatever is missing.";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Ok,
    /// Worth knowing, nothing stops working.
    Note,
    /// Something padwight needs doesn't work.
    Problem,
}

pub struct Finding {
    pub level: Level,
    pub title: String,
    /// Why it matters; lines are printed as they are.
    pub detail: String,
    /// Shell commands that fix it.
    pub fix: Option<String>,
}

impl Finding {
    fn new(level: Level, title: impl Into<String>, detail: impl Into<String>) -> Self {
        Finding { level, title: title.into(), detail: detail.into(), fix: None }
    }

    fn fix(mut self, commands: String) -> Self {
        self.fix = Some(commands);
        self
    }
}

/// Where the checks look. The tests point it at a made-up tree.
pub struct Roots {
    pub uinput: PathBuf,
    pub input_dev: PathBuf,
    pub sysfs_input: PathBuf,
    pub rule_dirs: Vec<PathBuf>,
    pub module_dirs: Vec<PathBuf>,
}

impl Roots {
    pub fn system() -> Self {
        Roots {
            uinput: "/dev/uinput".into(),
            input_dev: "/dev/input".into(),
            sysfs_input: "/sys/class/input".into(),
            rule_dirs: devices::rule_dirs(),
            module_dirs: ["/etc/modules-load.d", "/run/modules-load.d", "/usr/lib/modules-load.d", "/lib/modules-load.d"]
                .into_iter()
                .map(PathBuf::from)
                .collect(),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
enum RuleState {
    Missing,
    Current,
    /// Installed, but not what this version ships.
    Outdated,
}

fn file_state(dirs: &[PathBuf], name: &str, shipped: &str) -> RuleState {
    for dir in dirs {
        if let Ok(text) = fs::read_to_string(dir.join(name)) {
            return if rules(&text) == rules(shipped) { RuleState::Current } else { RuleState::Outdated };
        }
    }
    RuleState::Missing
}

/// The lines of a rules file that do something: its comments don't count when comparing copies.
fn rules(text: &str) -> Vec<&str> {
    text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).collect()
}

/// A command that writes `shipped`'s rules to `path` as root. printf and single quotes work the same
/// in bash, zsh and fish (which has no heredocs); the shipped files hold no single quote.
fn write_as_root(path: &str, shipped: &str) -> String {
    let lines: Vec<String> = rules(shipped).iter().map(|l| format!("'{l}'")).collect();
    format!("printf '%s\\n' {} | sudo tee {path} > /dev/null", lines.join(" "))
}

/// Commands that write a udev rule where udev reads it, and make udev apply it now.
fn install_rule(name: &str, shipped: &str) -> String {
    format!("{}\n{RELOAD}", write_as_root(&format!("/etc/udev/rules.d/{name}"), shipped))
}

const RELOAD: &str = "sudo udevadm control --reload && sudo udevadm trigger";

/// The checks that decide whether the daemon can do its job.
pub fn check_permissions(roots: &Roots) -> Vec<Finding> {
    let mut findings = vec![check_uinput(roots)];
    findings.extend(check_controllers(roots));
    findings
}

fn check_uinput(roots: &Roots) -> Finding {
    const WHY: &str = "padwight shows games a virtual gamepad, and sends keys and clicks through a virtual keyboard and mouse. \
The kernel creates virtual input devices only through /dev/uinput, so the daemon must open it for writing.";
    match fs::OpenOptions::new().write(true).open(&roots.uinput) {
        Ok(_) => Finding::new(Level::Ok, "Virtual devices: /dev/uinput is writable", ""),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let loaded = file_state(&roots.module_dirs, MODULES_CONF_NAME, MODULES_CONF);
            let note = if loaded == RuleState::Missing {
                "The uinput kernel module isn't loaded, and nothing loads it at boot."
            } else {
                "The uinput kernel module isn't loaded, though it is set to load at boot. This kernel may not have it."
            };
            let fix = format!(
                "sudo modprobe uinput\n{}",
                write_as_root(&format!("/etc/modules-load.d/{MODULES_CONF_NAME}"), MODULES_CONF)
            );
            Finding::new(Level::Problem, "Virtual devices: /dev/uinput doesn't exist", format!("{note}\n{WHY}")).fix(fix)
        }
        Err(e) if e.kind() == ErrorKind::PermissionDenied => uinput_denied(roots, WHY),
        Err(e) => Finding::new(Level::Problem, "Virtual devices: /dev/uinput can't be opened", format!("{e}\n{WHY}")),
    }
}

fn uinput_denied(roots: &Roots, why: &str) -> Finding {
    let state = file_state(&roots.rule_dirs, UINPUT_RULE_NAME, UINPUT_RULE);
    let (note, fix) = if state == RuleState::Current {
        (
            format!(
                "{UINPUT_RULE_NAME} is installed, but not yet applied to your session. Apply it, then log out and back in."
            ),
            RELOAD.to_owned(),
        )
    } else {
        (
            format!(
                "Your user can't write to /dev/uinput. {UINPUT_RULE_NAME} tags it for systemd-logind's `uaccess`, \
which gives access only to the user at the machine."
            ),
            install_rule(UINPUT_RULE_NAME, UINPUT_RULE),
        )
    };
    Finding::new(Level::Problem, "Virtual devices: no write access to /dev/uinput", format!("{note}\n{why}")).fix(fix)
}

fn check_controllers(roots: &Roots) -> Vec<Finding> {
    let nodes = devices::scan(&roots.sysfs_input, &roots.input_dev);
    let mut findings: Vec<Finding> = nodes.iter().filter_map(|n| check_node(roots, n)).collect();
    if !nodes.iter().any(|n| n.gamepad) {
        findings.push(Finding::new(
            Level::Note,
            "No controller is connected",
            "Connect one, then run this again to check that padwight can open it.",
        ));
    }
    findings.push(check_motion_rule(roots, &nodes));
    findings.push(check_hidraw_rule(roots, &nodes));
    findings
}

fn check_node(roots: &Roots, node: &Node) -> Option<Finding> {
    let Node { name, event, .. } = node;
    if node.is_pad_motion() {
        return Some(match node.access {
            Ok(()) => Finding::new(Level::Ok, format!("Gyro: {name} ({event}) is readable"), ""),
            Err(_) if node.denied() => motion_denied(roots, node),
            Err(kind) => Finding::new(Level::Problem, format!("Gyro: {name} ({event}) can't be opened: {kind}"), ""),
        });
    }
    if !node.gamepad {
        return None;
    }
    Some(match node.access {
        Ok(()) => Finding::new(Level::Ok, format!("Controller: {name} ({event}) is readable"), ""),
        Err(_) if node.denied() => gamepad_denied(node),
        Err(kind) => Finding::new(Level::Problem, format!("Controller: {name} ({event}) can't be opened: {kind}"), ""),
    })
}

fn gamepad_denied(node: &Node) -> Finding {
    let detail = "padwight reads the controller from its /dev/input node, and takes it exclusively so games see only the virtual \
gamepad. systemd-logind normally gives the user at the machine access to joysticks, but not over SSH, in some containers, \
or on systems without it.";
    Finding::new(Level::Problem, format!("Controller: no read access to {} ({})", node.name, node.event), detail)
        .fix("sudo usermod -aG input $USER\n# then log out and back in".to_owned())
}

fn motion_denied(roots: &Roots, node: &Node) -> Finding {
    let why = "Gyro comes from a separate input node, classed as an accelerometer, which systemd's rules leave root-only. \
The rule gives the user at the machine access to PlayStation and Nintendo controllers' motion sensors only, so a laptop's \
tilt sensor is untouched. Without it, everything but gyro works.";
    let state = file_state(&roots.rule_dirs, MOTION_RULE_NAME, MOTION_RULE);
    let (note, fix) = if state == RuleState::Current {
        ("The rule is installed, but not yet applied to this controller. Apply it, then reconnect the controller.", RELOAD.to_owned())
    } else {
        ("The motion-sensor rule isn't installed.", install_rule(MOTION_RULE_NAME, MOTION_RULE))
    };
    Finding::new(
        Level::Problem,
        format!("Gyro: no read access to the motion sensor of {} ({})", node.name, node.event),
        format!("{note}\n{why}"),
    )
    .fix(fix)
}

/// Reports whether the motion-sensor rule is installed. A missing rule is a problem only when a
/// pad that needs it is connected (that is reported by its node), so here it's a note.
fn check_motion_rule(roots: &Roots, nodes: &[Node]) -> Finding {
    match file_state(&roots.rule_dirs, MOTION_RULE_NAME, MOTION_RULE) {
        RuleState::Current => Finding::new(Level::Ok, format!("Gyro: {MOTION_RULE_NAME} is installed"), ""),
        RuleState::Outdated => Finding::new(
            Level::Note,
            format!("Gyro: the installed {MOTION_RULE_NAME} differs from this version's"),
            "Install this version's to be sure gyro access keeps working.",
        )
        .fix(install_rule(MOTION_RULE_NAME, MOTION_RULE)),
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
fn check_hidraw_rule(roots: &Roots, nodes: &[Node]) -> Finding {
    match file_state(&roots.rule_dirs, HIDRAW_RULE_NAME, HIDRAW_RULE) {
        RuleState::Current => Finding::new(Level::Ok, format!("Raw controller access: {HIDRAW_RULE_NAME} is installed"), ""),
        RuleState::Outdated => Finding::new(
            Level::Note,
            format!("Raw controller access: the installed {HIDRAW_RULE_NAME} differs from this version's"),
            "Install this version's to be sure Steam and games keep reading those controllers through padwight.",
        )
        .fix(install_rule(HIDRAW_RULE_NAME, HIDRAW_RULE)),
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

/// Everything the settings window shows: the permissions, and whether there is a desktop.
pub fn check_all() -> Vec<Finding> {
    let mut findings = check_permissions(&Roots::system());
    findings.push(check_desktop());
    findings
}

fn check_desktop() -> Finding {
    let graphical = ["WAYLAND_DISPLAY", "DISPLAY"].iter().any(|v| std::env::var_os(v).is_some_and(|s| !s.is_empty()));
    if graphical {
        Finding::new(Level::Ok, "Desktop session: found", "")
    } else {
        Finding::new(
            Level::Note,
            "Desktop session: none found here",
            "The settings window and overlays need a graphical session (neither WAYLAND_DISPLAY nor DISPLAY is set). \
The daemon itself doesn't.",
        )
    }
}

fn check_daemon() -> Finding {
    match crate::ipc::request(&crate::ipc::Request::Status) {
        Ok(_) => Finding::new(Level::Ok, "Daemon: running", ""),
        Err(_) => daemon_down(),
    }
}

fn daemon_down() -> Finding {
    let unit = home_unit_exists();
    let fix = if unit { "systemctl --user enable --now padwight" } else { "padwight daemon" };
    let detail = if unit {
        "Start it now and at every login."
    } else {
        "No padwight.service is installed for your user. Start it by hand, or install the unit (the release tarball's install.sh does)."
    };
    Finding::new(Level::Note, "Daemon: not running", detail).fix(fix.to_owned())
}

fn home_unit_exists() -> bool {
    dirs::config_dir()
        .map(|d| d.join("systemd/user"))
        .into_iter()
        .chain(["/etc/systemd/user", "/usr/lib/systemd/user"].map(PathBuf::from))
        .any(|dir| dir.join("padwight.service").exists())
}

/// Turns findings into the text `padwight doctor` prints.
pub fn render(findings: &[Finding]) -> String {
    let mut out = String::new();
    for f in findings {
        let mark = match f.level {
            Level::Ok => "✓",
            Level::Note => "•",
            Level::Problem => "✗",
        };
        out.push_str(&format!("{mark} {}\n", f.title));
        for line in f.detail.lines().filter(|l| !l.is_empty()) {
            out.push_str(&format!("    {line}\n"));
        }
        if let Some(fix) = &f.fix {
            out.push_str("    To fix, run:\n");
            for line in fix.lines() {
                out.push_str(&format!("        {line}\n"));
            }
        }
    }
    out
}

/// The problems that stop the daemon or a feature from working, as `padwight doctor` prints them.
pub fn problems_report() -> String {
    let problems: Vec<Finding> = check_permissions(&Roots::system()).into_iter().filter(|f| f.level == Level::Problem).collect();
    render(&problems)
}

#[expect(clippy::print_stdout, reason = "CLI output")]
pub fn run(args: &[&str]) -> Result<()> {
    match args {
        ["--explain"] => {
            println!("{REFERENCE}");
            return Ok(());
        }
        [] => {}
        _ => bail!("usage: padwight doctor [--explain]"),
    }
    let mut findings = check_permissions(&Roots::system());
    findings.push(check_desktop());
    findings.push(check_daemon());
    print!("{}", render(&findings));
    let problems = findings.iter().filter(|f| f.level == Level::Problem).count();
    match problems {
        0 => println!("\nEverything padwight needs is in place."),
        n => {
            println!("\nRun `padwight doctor --explain` for what each of these is for.");
            bail!("{n} problem{} found", if n == 1 { "" } else { "s" });
        }
    }
    Ok(())
}
