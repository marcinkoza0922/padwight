//! `padwight debug focus`: what the daemon makes of a window or a process, without a controller
//! or a session. It answers "why didn't my game's setup switch on?" from the same code the daemon
//! runs.

use anyhow::{Context, Result, bail};

use crate::{
    config::Config,
    focus,
    ipc::{self, Request, Response, WindowInfo},
};

pub const USAGE: &str = "  focus                          what the running daemon sees in front: the window, its
                                 program and the rule it matches
  focus <pid> [class] [title]    the same for a window that reported this pid and class, as the
                                 daemon would identify it now (a sandboxed game's pid is found)
  focus processes                running programs that a rule matches, as the process scan
                                 picks them (for desktops without focus tracking)
";

#[expect(clippy::print_stdout, reason = "CLI output")]
pub fn run(args: &[&str]) -> Result<()> {
    let config = Config::load().context("loading the config")?;
    match args {
        [] => from_daemon(&config)?,
        ["processes"] => processes(&config),
        [pid, rest @ ..] => {
            let pid: u32 = pid.parse().with_context(|| format!("{pid} is not a pid"))?;
            let class = rest.first().copied().unwrap_or_default();
            let title = rest.get(1..).map(|t| t.join(" ")).unwrap_or_default();
            let window = focus::identify(class.into(), title, pid);
            if window.pid != pid {
                println!("pid      {pid} names a process in a sandbox; here it is {}", window.pid);
            }
            println!("{}", explain(&config, &window));
        }
    }
    Ok(())
}

/// The daemon's own view, from its status.
#[expect(clippy::print_stdout, reason = "CLI output")]
fn from_daemon(config: &Config) -> Result<()> {
    let Response::Status(status) = ipc::request(&Request::Status)? else { bail!("the daemon didn't send its status") };
    println!("tracking {}", status.focus_backend.label());
    let game = status.active_game.as_deref().unwrap_or("General");
    println!("profile  {} ({game})", status.active_profile);
    match &status.focused {
        Some(window) => println!("{}", explain(config, window)),
        None => println!("window   nothing has focus (or the desktop doesn't say)"),
    }
    Ok(())
}

/// Each line the daemon's decisions rest on, for one window.
fn explain(config: &Config, w: &WindowInfo) -> String {
    let mut lines = vec![
        format!("window   class {:?}  title {:?}  pid {}", w.class, w.title, w.pid),
        format!("program  {}  path {:?}", w.exe, w.path),
    ];
    if let Some(app) = &w.steam_app_id {
        lines.push(format!("steam    app {app}"));
    }
    lines.push(match focus::matching_rule(config, w) {
        Some((game, rule)) => format!("rule     {game}: {} → profile {}", focus::rule_text(rule), rule.profile),
        None => "rule     none matches".to_owned(),
    });
    lines.extend(sdl_environment(w.pid).into_iter().map(|v| format!("env      {v}")));
    lines.join("\n")
}

/// The SDL controller variables a process runs with, which decide which controllers its SDL
/// shows the game (Steam's `SDL_GAMECONTROLLER_IGNORE_DEVICES`, say).
#[cfg(target_os = "linux")]
fn sdl_environment(pid: u32) -> Vec<String> {
    if pid == 0 {
        return Vec::new();
    }
    let environ = std::fs::read(format!("/proc/{pid}/environ")).unwrap_or_default();
    environ
        .split(|b| *b == 0)
        .map(String::from_utf8_lossy)
        .filter(|v| v.starts_with("SDL_GAMECONTROLLER") || v.starts_with("SDL_JOYSTICK"))
        .map(std::borrow::Cow::into_owned)
        .collect()
}

#[cfg(not(target_os = "linux"))]
fn sdl_environment(_pid: u32) -> Vec<String> {
    Vec::new()
}

#[expect(clippy::print_stdout, reason = "CLI output")]
fn processes(config: &Config) {
    let running = focus::running_processes();
    println!("{} processes, {} switched-on rules", running.len(), focus::rules(config).count());
    for (game, rule) in focus::rules(config) {
        let hits: Vec<String> = running.iter().filter(|p| focus::rule_matches_any(rule, std::slice::from_ref(p))).map(|p| format!("{} ({})", p.exe, p.pid)).collect();
        if !hits.is_empty() {
            println!("{game}: {} matches {}", focus::rule_text(rule), hits.join(", "));
        }
    }
    match focus::matching_process(config, &running) {
        Some((game, rule, p)) => println!("picked   {game}: {} for {} ({})", focus::rule_text(rule), p.exe, p.pid),
        None => println!("picked   nothing; the default profile applies"),
    }
}
