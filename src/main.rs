mod capture;
mod clock;
mod clipboard;
mod config;
mod controller_id;
mod daemon;
mod debug;
#[cfg(target_os = "linux")]
mod doctor;
mod engine;
mod faults;
mod figures;
mod focus;
mod assets;
mod font;
mod gui;
mod guide_menu;
mod tools;
mod icon;
mod icons;
mod info;
mod inputlog;
mod input;
mod ipc;
mod keyboard;
mod keycode;
mod keyword_icon;
mod launchers;
mod library;
mod media;
mod new_pad;
mod offer;
mod opener_watch;
mod menu;
mod nav;
mod monitor;
mod motion;
mod output;
mod overlay;
mod pack;
mod pad_identity;
mod pad_svg;
mod pad_widget;
mod priority;
mod radial;
mod quit;
mod record;
mod rumble;
mod style;
mod layout_editor;
mod snapshot;
mod sound;
mod system_menu;
mod touchpad_widget;
mod tray;
mod update;

use anyhow::{Result, bail};

use ipc::{Request, Response};

const USAGE: &str = "\
usage: padwight [command]

commands:
  gui              open the settings window (default)
  daemon [--debug [--fake-clock]] run the background remapping service (--debug lets `debug --attach` inject controllers; --fake-clock makes time move only on `debug advance`)
  status           show daemon status and detected controllers
  doctor [--explain] check the permissions padwight needs and say how to fix them (Linux)
  update [--check] check GitHub for a newer release and install it (only a copy the release installer made)
  enable|disable   turn remapping on or off
  profile <name>   switch to a profile (in the current game, General, or any game)
  next-profile     switch to the next profile
  reload           re-read the config file
  overlay-toggle   open or close the on-screen keyboard overlay
  numpad-toggle    open or close the on-screen numpad overlay
  menu <name>      show an on-screen menu
  close-overlays   close every overlay and menu (bind it to a key for a way out)
  debug            virtual controller for development and tests (debug help)";

/// Says why the overlay window process is quitting, in a single write. It shares the daemon's log,
/// and the usual `Error: ...` of a failing `main` goes out in pieces that a line of the daemon's
/// own can land in the middle of.
fn report_in_one_write(to: &mut impl std::io::Write, why: &anyhow::Error) {
    let _ = to.write_all(format!("Error: {why:?}\n").as_bytes());
}

#[expect(clippy::print_stdout, reason = "CLI output")]
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] | ["gui"] => {
            let var = |v| std::env::var(v).ok();
            if let Some(why) = gui::missing_display(var("WAYLAND_DISPLAY").as_deref(), var("DISPLAY").as_deref()) {
                bail!(why);
            }
            gui::run().map_err(Into::into)
        }
        ["daemon"] => daemon::run(false, false),
        ["daemon", "--debug"] => daemon::run(true, false),
        ["daemon", "--debug", "--fake-clock"] => daemon::run(true, true),
        ["status"] => status(),
        #[cfg(target_os = "linux")]
        ["doctor", rest @ ..] => doctor::run(rest),
        ["update", rest @ ..] => update::run(rest),
        ["enable"] => send(&Request::SetEnabled(true)),
        ["disable"] => send(&Request::SetEnabled(false)),
        ["profile", name] => send(&Request::SetProfile(name.to_string())),
        ["next-profile"] => send(&Request::NextProfile),
        ["reload"] => send(&Request::Reload),
        ["overlay"] => overlay::run().map_err(|e| {
            report_in_one_write(&mut std::io::stderr(), &e);
            std::process::exit(1)
        }),
        ["overlay-toggle"] => send(&Request::ToggleOverlay),
        ["numpad-toggle"] => send(&Request::ToggleNumpad),
        ["menu", name] => send(&Request::OpenMenu(name.to_string())),
        ["close-overlays"] => send(&Request::CloseOverlays),
        ["debug", rest @ ..] => debug::run(rest),
        ["-h" | "--help" | "help"] => {
            println!("{USAGE}");
            Ok(())
        }
        _ => bail!("{USAGE}"),
    }
}

fn send(req: &Request) -> Result<()> {
    ipc::request(req)?;
    Ok(())
}

#[expect(clippy::print_stdout, reason = "CLI output")]
fn status() -> Result<()> {
    let Response::Status(s) = ipc::request(&Request::Status)? else {
        bail!("unexpected reply from daemon");
    };
    println!("enabled: {}", s.enabled);
    match &s.active_game {
        Some(game) => println!("profile: {} ({game})", s.active_profile),
        None => println!("profile: {}", s.active_profile),
    }
    println!("focus tracking: {}", s.focus_backend.label());
    if let Some(w) = &s.focused {
        let steam = w.steam_app_id.as_deref().map(|id| format!(", Steam {id}")).unwrap_or_default();
        println!("focused: {} (class {}{steam})", w.exe, w.class);
    }
    for d in s.devices {
        let state = if d.managed { "remapping" } else if d.ignored { "ignored" } else { "idle" };
        let triggers = if d.analog_triggers { "" } else { ", on/off triggers" };
        let rumble = if d.rumble { "" } else { ", no rumble" };
        println!("  {} [{}] {state}{triggers}{rumble}", d.name, d.path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Counts the writes it is given.
    #[derive(Default)]
    struct Writes(Vec<Vec<u8>>);
    impl std::io::Write for Writes {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.push(buf.to_vec());
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_quitting_overlay_process_says_why_in_one_write() {
        let why = anyhow::anyhow!("no compositor").context("the overlay window needs a Wayland session");
        let mut out = Writes::default();
        report_in_one_write(&mut out, &why);
        assert_eq!(out.0.len(), 1);
        let text = String::from_utf8(out.0.remove(0)).unwrap();
        assert!(text.starts_with("Error: the overlay window needs a Wayland session\n") && text.contains("no compositor") && text.ends_with('\n'));
    }
}
