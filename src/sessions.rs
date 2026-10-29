//! Launch sessions: following the game a store starts after Play. The store's own link starts
//! it, so the game is found among the processes that appear afterwards: Steam's `reaper` names
//! the game on its command line, Heroic and Lutris put its name in the environment of the
//! processes they start. Only processes that appeared after Play are looked at.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::config::{Store, Title};

/// How long a game gets to show up before the session gives up: the store may be updating it,
/// or the user may have cancelled.
pub const FIND_TIMEOUT: Duration = Duration::from_secs(120);
/// How often a session looks for new processes while it waits for its game, and while it runs.
pub const WAITING_POLL: Duration = Duration::from_millis(250);
pub const RUNNING_POLL: Duration = Duration::from_secs(1);
/// How long a new process is looked at again: between `fork` and `exec` it still has its
/// parent's command line and environment.
const SETTLE: Duration = Duration::from_secs(1);

/// The system's processes, read from a `/proc` folder (a fake one in tests).
pub struct Procs {
    root: PathBuf,
}

impl Procs {
    pub fn system() -> Self {
        Procs { root: PathBuf::from("/proc") }
    }

    fn pids(&self) -> Vec<u32> {
        let Ok(entries) = fs::read_dir(&self.root) else { return Vec::new() };
        entries.flatten().filter_map(|e| e.file_name().to_str()?.parse().ok()).collect()
    }

    /// The process's state, parent and start time, from its `stat` (the name may hold spaces
    /// and parentheses, so fields are counted from its closing one).
    fn stat(&self, pid: u32) -> Option<Stat> {
        let text = fs::read_to_string(self.root.join(pid.to_string()).join("stat")).ok()?;
        let mut fields = text[text.rfind(')')? + 1..].split_whitespace();
        let state = fields.next()?.chars().next()?;
        let parent = fields.next()?.parse().ok()?;
        let started = fields.nth(17)?.parse().ok()?;
        Some(Stat { state, parent, started })
    }

    fn cmdline(&self, pid: u32) -> Vec<String> {
        let bytes = fs::read(self.root.join(pid.to_string()).join("cmdline")).unwrap_or_default();
        bytes.split(|b| *b == 0).filter(|a| !a.is_empty()).map(|a| String::from_utf8_lossy(a).into_owned()).collect()
    }

    /// One variable of the process's environment (only readable for our own processes).
    fn env_var(&self, pid: u32, key: &str) -> Option<String> {
        let bytes = fs::read(self.root.join(pid.to_string()).join("environ")).ok()?;
        let prefix = format!("{key}=");
        bytes.split(|b| *b == 0).find_map(|entry| String::from_utf8_lossy(entry).strip_prefix(&prefix).map(str::to_string))
    }

    fn has_dir(&self, pid: u32) -> bool {
        self.root.join(pid.to_string()).is_dir()
    }
}

struct Stat {
    state: char,
    parent: u32,
    started: u64,
}

/// A process, told from a later one that got its number by when it started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Proc {
    pid: u32,
    started: u64,
}

impl Proc {
    fn alive(self, procs: &Procs) -> bool {
        procs.stat(self.pid).is_some_and(|s| s.started == self.started && s.state != 'Z' && s.state != 'X')
    }
}

/// What names a title's game among the processes.
#[derive(Debug, Clone, PartialEq)]
enum Follow {
    /// `reaper SteamLaunch AppId=<id>`, with any of these ids (a shortcut's is its `appid`).
    Steam(Vec<String>),
    /// `HEROIC_APP_NAME` (and the runner, when both are known) in the environment.
    Heroic { app: String, runner: String },
    /// `GAME_NAME` in the environment.
    Lutris(String),
    /// Started by `padwight run`, whose process is the root: nothing else names the game.
    Wrapper,
}

/// The store variables `padwight run` passes on, and what each says about the game.
pub const STORE_VARS: [&str; 6] = ["SteamAppId", "SteamGameId", "STEAM_COMPAT_APP_ID", "HEROIC_APP_NAME", "HEROIC_APP_RUNNER", "GAME_NAME"];

/// Whether the store variables name this title's game.
pub fn names_title(title: &Title, vars: &[(String, String)]) -> bool {
    let get = |key: &str| vars.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());
    match Follow::of(title) {
        Follow::Steam(ids) => ["SteamAppId", "SteamGameId", "STEAM_COMPAT_APP_ID"].iter().filter_map(|k| get(k)).any(|v| ids.iter().any(|i| i == v)),
        Follow::Heroic { app, runner } => get("HEROIC_APP_NAME") == Some(app.as_str()) && (runner.is_empty() || get("HEROIC_APP_RUNNER").is_none_or(|r| r == runner)),
        Follow::Lutris(name) => get("GAME_NAME").is_some_and(|n| n.trim() == name),
        Follow::Wrapper => false,
    }
}

impl Follow {
    fn of(title: &Title) -> Self {
        match title.source {
            Store::Steam => {
                // A non-Steam shortcut's link id has its `appid` in the top 32 bits.
                let shortcut = title.store_id.parse::<u64>().ok().filter(|id| id & 0xffff_ffff == 0x0200_0000).map(|id| (id >> 32).to_string());
                Follow::Steam(std::iter::once(title.store_id.clone()).chain(shortcut).collect())
            }
            Store::Heroic => Follow::Heroic { app: title.store_id.clone(), runner: title.runner.clone() },
            Store::Lutris => Follow::Lutris(title.name.trim().to_string()),
            Store::Run => Follow::Wrapper,
        }
    }

    fn matches(&self, procs: &Procs, pid: u32) -> bool {
        match self {
            Follow::Steam(ids) => is_game_reaper(&procs.cmdline(pid), ids),
            Follow::Heroic { app, runner } => {
                procs.env_var(pid, "HEROIC_APP_NAME").is_some_and(|name| &name == app)
                    && (runner.is_empty() || procs.env_var(pid, "HEROIC_APP_RUNNER").is_none_or(|r| &r == runner))
            }
            Follow::Lutris(name) => procs.env_var(pid, "GAME_NAME").is_some_and(|n| n.trim() == name),
            Follow::Wrapper => false,
        }
    }
}

/// `reaper SteamLaunch AppId=<id> -- …`, but not its first form `… AppId=<id> Install=1 --`,
/// which sets the game's runtime up and exits before the game starts.
fn is_game_reaper(argv: &[String], ids: &[String]) -> bool {
    let Some(first) = argv.first() else { return false };
    if first.rsplit('/').next() != Some("reaper") || argv.get(1).map(String::as_str) != Some("SteamLaunch") {
        return false;
    }
    let head: Vec<&str> = argv[2..].iter().map(String::as_str).take_while(|a| *a != "--").collect();
    !head.contains(&"Install=1") && head.iter().any(|a| a.strip_prefix("AppId=").is_some_and(|id| ids.iter().any(|i| i == id)))
}

/// What a poll saw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Nothing,
    /// The game's processes were found.
    Found,
    /// Every process of the game is gone.
    Ended,
    /// No game showed up in time.
    TimedOut,
}

/// A game started from the Games page, being waited for and then followed.
pub struct Session {
    pub title: Title,
    began: Instant,
    follow: Follow,
    /// Processes that existed before (`None`), or when each new one was first seen.
    seen: HashMap<u32, Option<Instant>>,
    /// The game's processes: its root, what carries its name and what those started.
    members: Vec<Proc>,
    root: Option<Proc>,
    /// Window classes seen for the game's windows.
    classes: Vec<String>,
    /// The `padwight run` process the game came through, if any.
    wrapper: Option<u32>,
}

impl Session {
    /// Starts a session. Call it before opening the store's link, so only what the link starts is looked at.
    pub fn new(title: Title, procs: &Procs, now: Instant) -> Self {
        let follow = Follow::of(&title);
        Session { title, began: now, follow, seen: procs.pids().into_iter().map(|p| (p, None)).collect(), members: Vec::new(), root: None, classes: Vec::new(), wrapper: None }
    }

    /// A session for a game `padwight run` started: the wrapper is its root and everything the
    /// game starts is under it. `None` when the wrapper is already gone.
    pub fn wrapped(title: Title, procs: &Procs, pid: u32, now: Instant) -> Option<Self> {
        let started = procs.stat(pid)?.started;
        let wrapper = Proc { pid, started };
        let mut session = Session::new(title, procs, now);
        session.follow = Follow::Wrapper;
        session.members.push(wrapper);
        session.root = Some(wrapper);
        session.wrapper = Some(pid);
        Some(session)
    }

    /// The wrapper of a game this session already waits for: its tree joins the game's, and it
    /// becomes the root when nothing else was found.
    pub fn attach(&mut self, procs: &Procs, pid: u32) -> bool {
        let Some(stat) = procs.stat(pid) else { return false };
        let wrapper = Proc { pid, started: stat.started };
        if !self.members.iter().any(|m| m.pid == pid) {
            self.members.push(wrapper);
        }
        self.root.get_or_insert(wrapper);
        self.wrapper = Some(pid);
        true
    }

    /// The pid of the `padwight run` process of this game, if it came through one.
    pub fn wrapper(&self) -> Option<u32> {
        self.wrapper
    }

    /// Whether the game is the wrapper with this pid and everything under it.
    pub fn rooted_in(&self, pid: u32) -> bool {
        self.wrapper == Some(pid) && self.root.is_some_and(|r| r.pid == pid)
    }

    pub fn running(&self) -> bool {
        self.root.is_some()
    }

    pub fn next_poll(&self) -> Instant {
        self.began + if self.running() { RUNNING_POLL } else { WAITING_POLL }
    }

    /// Looks at the processes that appeared since the last poll.
    pub fn poll(&mut self, procs: &Procs, now: Instant) -> Event {
        let pids = procs.pids();
        let live: HashSet<u32> = pids.iter().copied().collect();
        self.seen.retain(|p, _| live.contains(p));
        // New processes, and the ones that appeared just now and may not have exec'd yet.
        let mut look: Vec<u32> = pids
            .into_iter()
            .filter(|p| match self.seen.get(p) {
                None => true,
                Some(first) => first.is_some_and(|at| now.duration_since(at) < SETTLE),
            })
            .filter(|p| !self.members.iter().any(|m| m.pid == *p))
            .collect();
        look.sort_unstable();
        for pid in &look {
            self.seen.entry(*pid).or_insert(Some(now));
        }
        let new = look;

        let was_running = self.running();
        for pid in new {
            let Some(stat) = procs.stat(pid) else { continue };
            let process = Proc { pid, started: stat.started };
            let child = self.members.iter().any(|m| m.pid == stat.parent);
            if child || self.follow.matches(procs, pid) {
                self.members.push(process);
                if self.root.is_none() {
                    self.root = Some(process);
                }
            }
        }
        if let Some(root) = self.root {
            let gone = match self.follow {
                // Everything the game starts stays under its `reaper`.
                Follow::Steam(_) | Follow::Wrapper => !root.alive(procs),
                _ => self.members.iter().all(|m| !m.alive(procs)),
            };
            return if gone { Event::Ended } else if was_running { Event::Nothing } else { Event::Found };
        }
        if now.duration_since(self.began) > FIND_TIMEOUT { Event::TimedOut } else { Event::Nothing }
    }

    /// Whether the process `pid` is the game's or something it started: `None` where it can't
    /// be told (the pid isn't one of ours, as in the Flatpak).
    pub fn owns(&self, procs: &Procs, pid: u32) -> Option<bool> {
        if !self.running() || pid == 0 || !procs.has_dir(pid) {
            return None;
        }
        let (mut at, mut steps) = (pid, 0);
        while at > 1 && steps < 64 {
            if self.members.iter().any(|m| m.pid == at) {
                return Some(true);
            }
            at = procs.stat(at)?.parent;
            steps += 1;
        }
        Some(false)
    }

    /// Whether a window with this class is the game's: Steam names it, and one learned from a
    /// window that was the game's by pid counts too.
    pub fn knows_class(&self, class: &str) -> bool {
        !class.is_empty() && (self.classes.iter().any(|c| c == class) || matches!(&self.follow, Follow::Steam(ids) if ids.iter().any(|i| class == format!("steam_app_{i}"))))
    }

    pub fn learn_class(&mut self, class: &str) {
        if !class.is_empty() && !self.classes.iter().any(|c| c == class) {
            self.classes.push(class.to_string());
        }
    }

    /// The process whose tree to end to end the game: the one of its top processes with the most
    /// under it (a Heroic game's launcher stub is a top process too, but ends with the game).
    pub fn kill_root(&self, procs: &Procs) -> Option<u32> {
        let live: Vec<u32> = self.members.iter().filter(|m| m.alive(procs)).map(|m| m.pid).collect();
        let tops = live.iter().copied().filter(|p| procs.stat(*p).is_some_and(|s| !live.contains(&s.parent)));
        tops.max_by_key(|top| live.iter().filter(|p| self.climbs_to(procs, **p, *top)).count())
    }

    fn climbs_to(&self, procs: &Procs, from: u32, top: u32) -> bool {
        let (mut at, mut steps) = (from, 0);
        while at > 1 && steps < 64 {
            if at == top {
                return true;
            }
            let Some(stat) = procs.stat(at) else { return false };
            at = stat.parent;
            steps += 1;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake `/proc`.
    struct Fake(PathBuf);

    impl Fake {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("sessions-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Fake(dir)
        }

        fn procs(&self) -> Procs {
            Procs { root: self.0.clone() }
        }

        fn add(&self, pid: u32, parent: u32, argv: &[&str], env: &[(&str, &str)]) {
            let dir = self.0.join(pid.to_string());
            fs::create_dir_all(&dir).unwrap();
            let stat = format!("{pid} (a (b) c) S {parent} 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 {} 0", 1000 + pid);
            fs::write(dir.join("stat"), stat).unwrap();
            fs::write(dir.join("cmdline"), argv.iter().map(|a| format!("{a}\0")).collect::<String>()).unwrap();
            fs::write(dir.join("environ"), env.iter().map(|(k, v)| format!("{k}={v}\0")).collect::<String>()).unwrap();
        }

        fn remove(&self, pid: u32) {
            fs::remove_dir_all(self.0.join(pid.to_string())).unwrap();
        }
    }

    impl Drop for Fake {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn title(source: Store, id: &str, runner: &str, name: &str) -> Title {
        Title { id: format!("{}-{id}", source.label()), name: name.into(), source, store_id: id.into(), runner: runner.into(), setup: String::new(), hidden: false, installed: true, env: Default::default(), hide_controllers: false }
    }

    const REAPER: [&str; 6] = ["/home/me/.local/share/Steam/ubuntu12_32/reaper", "SteamLaunch", "AppId=2379780", "--", "entry", "run"];

    #[test]
    fn a_steam_game_is_its_reaper_and_everything_under_it_even_after_a_double_fork() {
        let fake = Fake::new("steam");
        fake.add(1, 0, &["init"], &[]);
        fake.add(50, 1, &["steam"], &[]);
        // Before Play: an old game's reaper doesn't count, nor does the setup one.
        fake.add(60, 50, &["reaper", "SteamLaunch", "AppId=400", "--"], &[]);
        let (procs, now) = (fake.procs(), Instant::now());
        let mut session = Session::new(title(Store::Steam, "2379780", "", "Balatro"), &procs, now);
        assert_eq!(session.poll(&procs, now), Event::Nothing);

        fake.add(70, 50, &["reaper", "SteamLaunch", "AppId=2379780", "Install=1", "--", "setup"], &[]);
        assert_eq!(session.poll(&procs, now), Event::Nothing, "the setup reaper isn't the game");
        fake.remove(70);

        fake.add(80, 50, &REAPER, &[]);
        assert_eq!(session.poll(&procs, now), Event::Found);
        assert!(session.running());
        // The game double-forks: the middle process exits and the game is re-parented.
        fake.add(81, 80, &["wine"], &[]);
        assert_eq!(session.poll(&procs, now), Event::Nothing);
        fake.add(82, 81, &["Balatro.exe"], &[]);
        fake.remove(81);
        // The kernel hands an orphan to the nearest subreaper, which is `reaper`.
        fake.add(82, 80, &["Balatro.exe"], &[]);
        fake.add(83, 82, &["helper"], &[]);
        assert_eq!(session.poll(&procs, now), Event::Nothing);
        assert_eq!(session.owns(&procs, 82), Some(true), "re-parented to the reaper, still under it");
        assert_eq!(session.owns(&procs, 83), Some(true));
        assert_eq!(session.owns(&procs, 50), Some(false));
        assert_eq!(session.owns(&procs, 9999), None);
        assert_eq!(session.kill_root(&procs), Some(80));
        assert!(session.knows_class("steam_app_2379780") && !session.knows_class("steam_app_400"));

        fake.remove(80);
        assert_eq!(session.poll(&procs, now), Event::Ended);
    }

    #[test]
    fn a_shortcut_is_found_by_its_appid() {
        let fake = Fake::new("shortcut");
        fake.add(1, 0, &["init"], &[]);
        let (procs, now) = (fake.procs(), Instant::now());
        // A shortcut's link id has its `appid` (3135921720) in the top 32 bits.
        let id = (3_135_921_720_u64 << 32 | 0x0200_0000).to_string();
        let mut session = Session::new(title(Store::Steam, &id, "", "Cuphead"), &procs, now);
        fake.add(5, 1, &["reaper", "SteamLaunch", "AppId=3135921720", "--", "x"], &[]);
        assert_eq!(session.poll(&procs, now), Event::Found);
    }

    #[test]
    fn a_heroic_game_is_found_by_its_environment_away_from_heroics_own_tree() {
        let fake = Fake::new("heroic");
        fake.add(1, 0, &["init"], &[]);
        fake.add(10, 1, &["heroic"], &[("HEROIC_APP_NAME", "other")]);
        fake.add(11, 1, &["flatpak-portal"], &[]);
        let (procs, now) = (fake.procs(), Instant::now());
        let mut session = Session::new(title(Store::Heroic, "1963513391", "gog", "Cuphead"), &procs, now);
        let game = [("HEROIC_APP_NAME", "1963513391"), ("HEROIC_APP_RUNNER", "gog")];
        fake.add(20, 10, &["gogdl"], &game);
        fake.add(21, 20, &["umu_run"], &game);
        assert_eq!(session.poll(&procs, now), Event::Found);
        // The game itself is under the portal, not under gogdl.
        fake.add(30, 11, &["pv-adverb"], &game);
        fake.add(31, 30, &["proton"], &game);
        fake.add(32, 31, &["Cuphead.exe"], &game);
        assert_eq!(session.poll(&procs, now), Event::Nothing);
        assert_eq!(session.owns(&procs, 32), Some(true));
        assert_eq!(session.owns(&procs, 10), Some(false));
        assert_eq!(session.kill_root(&procs), Some(30), "the top with the game under it, not the launcher stub");

        // Ending one top isn't the end of the game.
        fake.remove(20);
        fake.remove(21);
        assert_eq!(session.poll(&procs, now), Event::Nothing);
        for pid in [30, 31, 32] {
            fake.remove(pid);
        }
        assert_eq!(session.poll(&procs, now), Event::Ended);
    }

    #[test]
    fn a_wrapped_game_is_everything_under_the_wrapper_and_ends_with_it() {
        let fake = Fake::new("wrapped");
        fake.add(1, 0, &["init"], &[]);
        fake.add(40, 1, &["padwight", "run"], &[]);
        let (procs, now) = (fake.procs(), Instant::now());
        let mut session = Session::wrapped(title(Store::Run, "", "", "game"), &procs, 40, now).unwrap();
        assert!(session.running() && session.rooted_in(40) && !session.rooted_in(41));
        fake.add(41, 40, &["game"], &[]);
        fake.add(42, 41, &["helper"], &[]);
        assert_eq!(session.poll(&procs, now), Event::Nothing);
        assert_eq!(session.owns(&procs, 42), Some(true));
        assert_eq!(session.kill_root(&procs), Some(40));
        // A process that double-forks is handed to the wrapper, a subreaper.
        fake.remove(41);
        fake.add(42, 40, &["helper"], &[]);
        assert_eq!(session.owns(&procs, 42), Some(true));
        fake.remove(40);
        assert_eq!(session.poll(&procs, now), Event::Ended);
        assert!(Session::wrapped(title(Store::Run, "", "", "game"), &procs, 40, now).is_none(), "a wrapper that is gone");
    }

    #[test]
    fn a_wrapper_joins_the_session_that_waits_for_its_game() {
        let fake = Fake::new("attach");
        fake.add(1, 0, &["init"], &[]);
        let (procs, now) = (fake.procs(), Instant::now());
        let mut session = Session::new(title(Store::Heroic, "7", "gog", "G"), &procs, now);
        fake.add(40, 1, &["padwight", "run"], &[]);
        assert!(session.attach(&procs, 40));
        assert!(session.running() && session.rooted_in(40));
        fake.add(41, 40, &["game"], &[]);
        assert_eq!(session.poll(&procs, now), Event::Nothing);
        assert_eq!(session.owns(&procs, 41), Some(true));
        assert!(!session.attach(&procs, 999));
    }

    #[test]
    fn the_stores_variables_name_a_title() {
        let vars = |pairs: &[(&str, &str)]| pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect::<Vec<_>>();
        let steam = title(Store::Steam, "2379780", "", "Balatro");
        assert!(names_title(&steam, &vars(&[("SteamGameId", "2379780")])));
        assert!(!names_title(&steam, &vars(&[("SteamGameId", "400")])));
        let shortcut = title(Store::Steam, &(3_135_921_720_u64 << 32 | 0x0200_0000).to_string(), "", "Cuphead");
        assert!(names_title(&shortcut, &vars(&[("SteamAppId", "3135921720")])));
        let heroic = title(Store::Heroic, "7", "gog", "G");
        assert!(names_title(&heroic, &vars(&[("HEROIC_APP_NAME", "7"), ("HEROIC_APP_RUNNER", "gog")])));
        assert!(!names_title(&heroic, &vars(&[("HEROIC_APP_NAME", "7"), ("HEROIC_APP_RUNNER", "legendary")])));
        assert!(names_title(&title(Store::Lutris, "1", "", "Rome "), &vars(&[("GAME_NAME", "Rome")])));
        assert!(!names_title(&title(Store::Run, "", "", "x"), &vars(&[("GAME_NAME", "x")])));
    }

    #[test]
    fn another_runner_or_game_is_not_followed() {
        let fake = Fake::new("heroic-other");
        fake.add(1, 0, &["init"], &[]);
        let (procs, now) = (fake.procs(), Instant::now());
        let mut session = Session::new(title(Store::Heroic, "7", "gog", "G"), &procs, now);
        fake.add(2, 1, &["a"], &[("HEROIC_APP_NAME", "7"), ("HEROIC_APP_RUNNER", "legendary")]);
        fake.add(3, 1, &["b"], &[("HEROIC_APP_NAME", "8"), ("HEROIC_APP_RUNNER", "gog")]);
        assert_eq!(session.poll(&procs, now), Event::Nothing);
    }

    #[test]
    fn a_lutris_game_is_found_by_its_name_and_a_silent_launch_times_out() {
        let fake = Fake::new("lutris");
        fake.add(1, 0, &["init"], &[]);
        let (procs, now) = (fake.procs(), Instant::now());
        let mut session = Session::new(title(Store::Lutris, "1", "", "Total War: Rome II "), &procs, now);
        assert_eq!(session.poll(&procs, now + FIND_TIMEOUT + Duration::from_secs(1)), Event::TimedOut);
        let mut session = Session::new(title(Store::Lutris, "1", "", "Total War: Rome II "), &procs, now);
        fake.add(9, 1, &["wine"], &[("GAME_NAME", "Total War: Rome II "), ("LUTRIS_GAME_UUID", "x")]);
        assert_eq!(session.poll(&procs, now), Event::Found);
    }

    #[test]
    fn a_process_that_reuses_a_number_is_not_the_game() {
        let fake = Fake::new("reuse");
        fake.add(1, 0, &["init"], &[]);
        let (procs, now) = (fake.procs(), Instant::now());
        let mut session = Session::new(title(Store::Steam, "2379780", "", "B"), &procs, now);
        fake.add(80, 1, &REAPER, &[]);
        assert_eq!(session.poll(&procs, now), Event::Found);
        // The reaper exits and an unrelated process gets pid 80 (a later start time).
        fake.remove(80);
        fake.add(80, 1, &["bash"], &[]);
        fs::write(fake.0.join("80/stat"), "80 (bash) S 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 9999 0").unwrap();
        assert_eq!(session.poll(&procs, now), Event::Ended);
    }

    #[test]
    fn reaper_command_lines_are_told_apart() {
        let argv = |s: &str| s.split(' ').map(str::to_string).collect::<Vec<_>>();
        let ids = ["10".to_string()];
        assert!(is_game_reaper(&argv("/x/reaper SteamLaunch AppId=10 -- run"), &ids));
        assert!(!is_game_reaper(&argv("/x/reaper SteamLaunch AppId=10 Install=1 -- run"), &ids));
        assert!(!is_game_reaper(&argv("/x/reaper SteamLaunch AppId=100 -- run"), &ids));
        // An id after `--` belongs to the game's own command line.
        assert!(!is_game_reaper(&argv("/x/reaper SteamLaunch -- game AppId=10"), &ids));
        assert!(!is_game_reaper(&argv("/x/steam SteamLaunch AppId=10 --"), &ids));
        assert!(!is_game_reaper(&[], &ids));
    }

    /// A real process named `reaper` with Steam's command line (a shell running a script called
    /// `SteamLaunch`) that starts a child: found, owning the child, stopped through its kill root.
    #[test]
    #[cfg(target_os = "linux")]
    fn follows_and_stops_a_real_fake_reaper() {
        use std::os::unix::process::CommandExt;
        let dir = std::env::temp_dir().join(format!("sessions-real-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SteamLaunch"), "sleep 60 &\necho $! > child\nwait\n").unwrap();
        let procs = Procs::system();
        let mut session = Session::new(title(Store::Steam, "4242", "", "Fake"), &procs, Instant::now());
        // Not the game: the setup form of the same command line.
        let mut setup = std::process::Command::new("sleep");
        setup.arg("60").arg0("reaper");
        let mut setup = setup.spawn().unwrap();
        let mut reaper = std::process::Command::new("sh");
        reaper.current_dir(&dir).arg0("reaper").args(["SteamLaunch", "AppId=4242", "--"]);
        let mut reaper = reaper.spawn().unwrap();

        let wait_for = |session: &mut Session, want: Event| {
            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline {
                if session.poll(&procs, Instant::now()) == want {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            false
        };
        assert!(wait_for(&mut session, Event::Found));
        let child: u32 = loop {
            if let Ok(text) = fs::read_to_string(dir.join("child")) && let Ok(pid) = text.trim().parse() {
                break pid;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let _ = session.poll(&procs, Instant::now());
        assert_eq!(session.owns(&procs, child), Some(true));
        assert_eq!(session.owns(&procs, setup.id()), Some(false));
        assert_eq!(session.kill_root(&procs), Some(reaper.id()));

        crate::quit::force_quit(session.kill_root(&procs).unwrap());
        reaper.wait().unwrap();
        assert!(wait_for(&mut session, Event::Ended));
        setup.kill().unwrap();
        setup.wait().unwrap();
        fs::remove_dir_all(dir).unwrap();
    }

    /// Plays a game for real, follows it and stops it: `cargo test sessions::tests::play_for_real -- --ignored --nocapture`
    /// (needs Steam or Heroic running, with `PADWIGHT_PLAY=steam:2379780` `heroic:gog:1963513391` or `lutris:<id>:<name>`).
    #[test]
    #[ignore = "starts a real game"]
    #[cfg(target_os = "linux")]
    #[expect(clippy::print_stdout, reason = "a manual check")]
    fn play_for_real() {
        let want = std::env::var("PADWIGHT_PLAY").expect("PADWIGHT_PLAY=steam:<id> or heroic:<runner>:<app name>");
        let parts: Vec<&str> = want.split(':').collect();
        let t = match parts[..] {
            ["steam", id] => title(Store::Steam, id, "", "Real game"),
            ["heroic", runner, id] => title(Store::Heroic, id, runner, "Real game"),
            ["lutris", id, name] => title(Store::Lutris, id, "", name),
            _ => panic!("bad PADWIGHT_PLAY"),
        };
        let procs = Procs::system();
        let mut session = Session::new(t.clone(), &procs, Instant::now());
        crate::titles::open_link(&t.link()).unwrap();
        let begun = Instant::now();
        let step = |session: &mut Session, want: Event, secs: u64| {
            let deadline = Instant::now() + Duration::from_secs(secs);
            while Instant::now() < deadline {
                if session.poll(&procs, Instant::now()) == want {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(250));
            }
            false
        };
        assert!(step(&mut session, Event::Found, 90), "never found");
        println!("found after {:?}: members {:?}", begun.elapsed(), session.members.len());
        std::thread::sleep(Duration::from_secs(25));
        let _ = session.poll(&procs, Instant::now());
        println!("members {} kill root {:?}", session.members.len(), session.kill_root(&procs));
        for m in session.members.iter().take(6) {
            println!("  {} {:?}", m.pid, procs.cmdline(m.pid).iter().take(3).collect::<Vec<_>>());
        }
        let kill = session.kill_root(&procs).unwrap();
        let stopped = Instant::now();
        crate::quit::force_quit(kill);
        assert!(step(&mut session, Event::Ended, 30), "exit not noticed");
        println!("ended {:?} after stop", stopped.elapsed());
    }
}
