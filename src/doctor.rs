//! `padwight doctor`: checks what the daemon needs from the system (access to /dev/uinput, to the
//! controllers and to their motion sensors, and the udev rules that grant it) and says what to
//! change, why, and the exact commands.

pub(crate) mod devices;
mod reference;
mod report;
mod scrub;
mod service;
mod system;
mod udev_rules;
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
    /// A system file is installed but differs from the one this version ships. An update leaves
    /// those alone, so `padwight update` ends by listing them.
    pub stale: bool,
}

impl Finding {
    fn new(level: Level, title: impl Into<String>, detail: impl Into<String>) -> Self {
        Finding { level, title: title.into(), detail: detail.into(), fix: None, stale: false }
    }

    fn fix(mut self, commands: String) -> Self {
        self.fix = Some(commands);
        self
    }

    fn stale(mut self) -> Self {
        self.stale = true;
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
    findings.extend(stale_uinput_files(roots));
    findings.extend(check_controllers(roots));
    findings
}

/// The uinput rule and the module list, when installed copies differ from this version's. (A missing
/// one is no finding: the desktop may grant access already, or the module be loaded another way.)
fn stale_uinput_files(roots: &Roots) -> Vec<Finding> {
    let mut found = Vec::new();
    if file_state(&roots.rule_dirs, UINPUT_RULE_NAME, UINPUT_RULE) == RuleState::Outdated {
        found.push(
            Finding::new(
                Level::Note,
                format!("Virtual devices: the installed {UINPUT_RULE_NAME} differs from this version's"),
                "Install this version's to be sure padwight keeps access to /dev/uinput.",
            )
            .fix(install_rule(UINPUT_RULE_NAME, UINPUT_RULE))
            .stale(),
        );
    }
    if file_state(&roots.module_dirs, MODULES_CONF_NAME, MODULES_CONF) == RuleState::Outdated {
        found.push(
            Finding::new(
                Level::Note,
                format!("Virtual devices: the installed {MODULES_CONF_NAME} differs from this version's"),
                "Install this version's to be sure the uinput module keeps loading at boot.",
            )
            .fix(write_as_root(&format!("/etc/modules-load.d/{MODULES_CONF_NAME}"), MODULES_CONF))
            .stale(),
        );
    }
    found
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
    findings.push(udev_rules::check_motion_rule(roots, &nodes));
    findings.push(udev_rules::check_hidraw_rule(roots, &nodes));
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

/// Everything the settings window shows: the permissions, the desktop and the helper programs.
pub fn check_all() -> Vec<Finding> {
    let mut findings = check_permissions(&Roots::system());
    findings.push(check_desktop());
    findings.push(clipboard_finding(crate::capture::on_path(crate::clipboard::TOOL)));
    findings
}

fn check_desktop() -> Finding {
    let set = |v: &str| std::env::var_os(v).is_some_and(|s| !s.is_empty());
    desktop_finding(set("WAYLAND_DISPLAY"), set("DISPLAY"))
}

/// What the session variables say: Wayland is needed, an X11 session alone isn't supported.
fn desktop_finding(wayland: bool, x11: bool) -> Finding {
    if wayland {
        Finding::new(Level::Ok, "Desktop session: Wayland", "")
    } else if x11 {
        Finding::new(
            Level::Problem,
            "Desktop session: X11, which isn't supported",
            "padwight needs a Wayland session for the settings window, the on-screen keyboard and the overlays \
(DISPLAY is set, WAYLAND_DISPLAY isn't). Log in to your desktop's Wayland session instead. Games running under Xwayland are fine.",
        )
    } else {
        Finding::new(
            Level::Note,
            "Desktop session: none found here",
            "The settings window and overlays need a Wayland session (WAYLAND_DISPLAY isn't set). The daemon itself doesn't.",
        )
    }
}

/// Whether the calculator's Copy can reach the clipboard. Nothing else needs it, so a missing
/// tool is a note; the package's name is the same on every major distro, the install command isn't.
fn clipboard_finding(found: bool) -> Finding {
    if found {
        Finding::new(Level::Ok, format!("Clipboard: {} found", crate::clipboard::TOOL), "")
    } else {
        Finding::new(
            Level::Note,
            format!("Clipboard: {} isn't installed", crate::clipboard::TOOL),
            "The calculator's Copy puts its answer on the clipboard through wl-copy. Install your distro's \
wl-clipboard package to use it. Everything else, Paste included, works without it.",
        )
    }
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

/// `padwight doctor --files`: only the installed system files that differ from this version's, with
/// the commands that need root. `padwight update` runs it on the new binary, since an update never
/// touches /etc itself. Prints nothing when all is current.
#[expect(clippy::print_stdout, reason = "CLI output")]
fn print_stale_files() -> Result<()> {
    let stale: Vec<Finding> = check_permissions(&Roots::system()).into_iter().filter(|f| f.stale).collect();
    if !stale.is_empty() {
        println!("This version changes system files that need root, so they weren't updated:");
        print!("{}", render(&stale));
    }
    Ok(())
}

#[expect(clippy::print_stdout, reason = "CLI output")]
pub fn run(args: &[&str]) -> Result<()> {
    match args {
        ["--files"] => return print_stale_files(),
        ["--explain"] => {
            println!("{}", reference::REFERENCE);
            return Ok(());
        }
        ["--report", file] => return report::run(file),
        [] => {}
        _ => bail!("usage: padwight doctor [--explain|--files|--report <file>]"),
    }
    let mut findings = check_all();
    findings.push(service::check_daemon());
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
