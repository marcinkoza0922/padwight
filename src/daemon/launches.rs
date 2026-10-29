//! Games started from the Games page: Play opens the store's link and a session follows the game
//! the store starts (see `sessions`). While one of its windows is in front, its setup is active.

use std::time::Instant;

use super::Daemon;
use crate::{
    config::{ProfileRef, Title},
    ipc::{LaunchInfo, Response, WindowInfo},
    monitor::log,
    sessions::{Event, Procs, Session},
    titles,
};

impl Daemon {
    /// Opens the title's link and starts looking for its game.
    pub(super) fn launch_title(&mut self, title: &Title) -> Response {
        if self.launch_sessions.iter().any(|s| s.title.id == title.id) {
            return Response::Error(format!("{} is already starting or running", title.name));
        }
        // Looked at before the link is opened, so only what the link starts counts.
        let session = Session::new(title.clone(), &Procs::system(), Instant::now());
        if let Err(e) = titles::open_link(&title.link()) {
            return Response::Error(e);
        }
        log!("launching {} through {}", title.name, title.source.label());
        self.launch_sessions.push(session);
        Response::Ok
    }

    /// Ends the game started for the title with this id.
    pub(super) fn stop_title(&mut self, id: &str) -> Response {
        let Some(session) = self.launch_sessions.iter().find(|s| s.title.id == id) else { return Response::Error("that game wasn't started from here".into()) };
        match session.kill_root(&Procs::system()) {
            Some(pid) => {
                log!("stopping {} (pid {pid})", session.title.name);
                crate::quit::force_quit(pid);
                Response::Ok
            }
            None => Response::Error(format!("{} hasn't started yet", session.title.name)),
        }
    }

    pub(super) fn launch_infos(&self) -> Vec<LaunchInfo> {
        self.launch_sessions.iter().map(|s| LaunchInfo { id: s.title.id.clone(), running: s.running() }).collect()
    }

    pub(super) fn launch_deadline(&self) -> Option<Instant> {
        self.launch_sessions.iter().map(Session::next_poll).min()
    }

    /// Looks for new processes in the sessions that are due, and closes the ones whose game
    /// ended or never showed up.
    pub(super) fn poll_launches(&mut self, now: Instant) {
        if self.launch_sessions.is_empty() {
            return;
        }
        let procs = Procs::system();
        let mut closed = Vec::new();
        for (i, session) in self.launch_sessions.iter_mut().enumerate() {
            if session.next_poll() > now {
                continue;
            }
            match session.poll(&procs, now) {
                Event::Nothing => {}
                Event::Found => log!("{} started", session.title.name),
                Event::Ended => closed.push((i, format!("{} exited", session.title.name))),
                Event::TimedOut => closed.push((i, format!("{} didn't start", session.title.name))),
            }
        }
        for (i, why) in closed.into_iter().rev() {
            let session = self.launch_sessions.remove(i);
            log!("{why}");
            if self.session_game.as_deref() == Some(session.title.setup.as_str()) && !session.title.setup.is_empty() {
                self.leave_game(&why);
            }
            if !session.running() {
                self.say(vec![why]);
            }
        }
    }

    /// The setup to play `window` with, when it's the window of a game started from here whose
    /// title has one: the setup's name and the game's root process.
    pub(super) fn session_setup(&mut self, window: &WindowInfo) -> Option<(String, u32)> {
        let procs = Procs::system();
        let session = self.launch_sessions.iter_mut().find(|s| s.running() && (s.owns(&procs, window.pid) == Some(true) || s.knows_class(&window.class)))?;
        session.learn_class(&window.class);
        let game = session.title.setup.clone();
        let pid = session.kill_root(&procs)?;
        (!game.is_empty() && self.config.games.iter().any(|g| g.name == game)).then_some((game, pid))
    }

    /// The profile a setup plays a game started from here with: its first rule's, else its first.
    pub(super) fn session_profile(&self, game: &str) -> Option<ProfileRef> {
        let setup = self.config.games.iter().find(|g| g.name == game)?;
        let profile = setup.rules.first().map(|r| r.profile.as_str()).or_else(|| setup.profiles.first().map(|p| p.name.as_str()))?;
        Some(ProfileRef::new(Some(game), profile))
    }

    /// What force quit ends when `window` is a game started from here: its name and the process
    /// whose tree is the game.
    pub(super) fn session_target(&self, window: &WindowInfo) -> Option<(String, u32)> {
        let procs = Procs::system();
        let session = self.launch_sessions.iter().find(|s| s.running() && (s.owns(&procs, window.pid) == Some(true) || s.knows_class(&window.class)))?;
        Some((session.title.name.clone(), session.kill_root(&procs)?))
    }
}
