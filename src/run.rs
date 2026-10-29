//! `padwight run [--title <id>] [--setup <name>] -- <command>…`: starts a game as its parent, so
//! the daemon knows its whole process tree. It goes in a store's launch options
//! (`padwight run -- %command%`). It never stops a game from starting: with no daemon, or one
//! that refuses, the command is run as if it weren't there.

use std::{
    io::{BufRead, BufReader, Write},
    os::unix::process::{CommandExt, ExitStatusExt},
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Result, bail};

use crate::{
    ipc::{self, Request, Response, RunReply, RunRequest, UnixStream},
    quit, sessions,
};

/// How often the supervisor looks at its children and for a signal.
const TICK: Duration = Duration::from_millis(50);
/// How long to wait before registering again with a daemon that went away.
const RECONNECT: Duration = Duration::from_secs(3);

const USAGE: &str = "usage: padwight run [--title <id>] [--setup <name>] [--] <command> [args…]";

/// A signal asked the game to end.
static TERMINATE: AtomicBool = AtomicBool::new(false);

#[derive(Debug, PartialEq)]
struct Args {
    title: Option<String>,
    setup: Option<String>,
    command: Vec<String>,
}

fn parse(args: &[&str]) -> Result<Args> {
    let (mut title, mut setup) = (None, None);
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match *arg {
            "--title" => title = Some(rest.next().ok_or_else(|| anyhow::anyhow!("--title needs an id\n{USAGE}"))?.to_string()),
            "--setup" => setup = Some(rest.next().ok_or_else(|| anyhow::anyhow!("--setup needs a name\n{USAGE}"))?.to_string()),
            "--" => break,
            first => return Ok(Args { title, setup, command: std::iter::once(first).chain(rest.copied()).map(str::to_string).collect() }),
        }
    }
    let command: Vec<String> = rest.map(|a| (*a).to_string()).collect();
    if command.is_empty() {
        bail!("{USAGE}");
    }
    Ok(Args { title, setup, command })
}

/// Runs the command and returns its exit status.
#[expect(clippy::print_stderr, reason = "CLI output, in a game's launch log")]
pub fn run(args: &[&str]) -> Result<i32> {
    let args = parse(args)?;
    let request = RunRequest {
        pid: std::process::id(),
        command: args.command.clone(),
        title: args.title,
        setup: args.setup,
        store_env: sessions::STORE_VARS.iter().filter_map(|k| Some(((*k).to_string(), std::env::var(k).ok()?))).collect(),
    };
    let (program, rest) = args.command.split_first().expect("parse needs a command");
    let mut command = Command::new(program);
    command.args(rest);
    let connection = match register(&request) {
        Ok((connection, reply)) => {
            command.envs(reply.env.iter().map(|(k, v)| (k, v)));
            keep_registered(connection, request);
            Some(reply.name)
        }
        Err(e) => {
            eprintln!("padwight: {e:#}; starting the game without it");
            None
        }
    };
    if connection.is_none() {
        // Nothing to tell anyone: be the command itself.
        return Err(command.exec().into());
    }
    supervise(command)
}

/// Tells the daemon about the game and reads what to add to its environment.
fn register(request: &RunRequest) -> Result<(UnixStream, RunReply)> {
    use anyhow::Context;
    let path = ipc::socket_path();
    let stream = UnixStream::connect(&path).with_context(|| format!("the daemon isn't reachable at {}", path.display()))?;
    stream.set_read_timeout(Some(ipc::REPLY_WAIT + Duration::from_secs(2)))?;
    let mut line = serde_json::to_string(&Request::Run(request.clone()))?;
    line.push('\n');
    (&stream).write_all(line.as_bytes())?;
    let mut reply = String::new();
    BufReader::new(&stream).read_line(&mut reply).context("no reply from the daemon")?;
    match serde_json::from_str(&reply).context("the daemon's reply")? {
        Response::Launched(reply) => {
            stream.set_read_timeout(None)?;
            Ok((stream, reply))
        }
        Response::Error(e) => bail!(e),
        _ => bail!("unexpected reply from the daemon"),
    }
}

/// Holds the connection, which is how the daemon knows the game is still there, and registers
/// again if the daemon goes away and comes back.
fn keep_registered(mut stream: UnixStream, request: RunRequest) {
    thread::spawn(move || loop {
        let mut sink = String::new();
        let mut reader = BufReader::new(&stream);
        while reader.read_line(&mut sink).is_ok_and(|n| n > 0) {
            sink.clear();
        }
        stream = loop {
            thread::sleep(RECONNECT);
            if let Ok((stream, _)) = register(&request) {
                break stream;
            }
        };
    });
}

extern "C" fn on_signal(_: libc::c_int) {
    TERMINATE.store(true, Ordering::SeqCst);
}

/// Starts the command as this process's child, reaps what the game leaves behind, and stays
/// until the command and everything it started is gone.
#[expect(clippy::print_stderr, reason = "CLI output, in a game's launch log")]
fn supervise(mut command: Command) -> Result<i32> {
    // SAFETY: only sets a flag for the kernel to apply to this process.
    unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) };
    for sig in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
        // SAFETY: the handler only stores to an atomic, which is allowed in a signal handler.
        unsafe { libc::signal(sig, on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t) };
    }
    let child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            eprintln!("padwight: cannot start {}: {e}", command.get_program().to_string_lossy());
            return Ok(127);
        }
    };
    let (main, me) = (child.id(), std::process::id());
    let mut status = None;
    let (mut asked, mut killed) = (None::<Instant>, false);
    loop {
        // Everything that is ours to reap: the command, and what it left behind.
        loop {
            let mut raw = 0;
            // SAFETY: waitpid writes the status into `raw`.
            let pid = unsafe { libc::waitpid(-1, &mut raw, libc::WNOHANG) };
            if pid <= 0 {
                break;
            }
            if pid as u32 == main {
                status = Some(std::process::ExitStatus::from_raw(raw));
            }
        }
        let left = descendants(me);
        if let Some(status) = status
            && left.is_empty()
        {
            return Ok(status.code().unwrap_or_else(|| 128 + status.signal().unwrap_or(0)));
        }
        if TERMINATE.load(Ordering::SeqCst) {
            if asked.is_none() {
                asked = Some(Instant::now());
                quit::signal(&left, libc::SIGTERM);
            } else if !killed && asked.is_some_and(|at| at.elapsed() > quit::GRACE) {
                killed = true;
                quit::signal(&left, libc::SIGKILL);
            }
        }
        thread::sleep(TICK);
    }
}

/// Everything under `me`: children first, since orphans are handed to this process.
fn descendants(me: u32) -> Vec<u32> {
    let parents = quit::parents();
    let mut found = vec![me];
    let mut i = 0;
    while i < found.len() {
        let parent = found[i];
        let mut children: Vec<u32> = parents.iter().filter(|(_, p)| **p == parent).map(|(c, _)| *c).collect();
        children.sort_unstable();
        children.retain(|c| !found.contains(c));
        found.extend(children);
        i += 1;
    }
    found.remove(0);
    found.reverse();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_come_before_the_command() {
        let args = parse(&["--title", "steam-1", "--setup", "Doom", "--", "echo", "--title"]).unwrap();
        assert_eq!(args, Args { title: Some("steam-1".into()), setup: Some("Doom".into()), command: vec!["echo".into(), "--title".into()] });
        let args = parse(&["game", "-a"]).unwrap();
        assert_eq!((args.title, args.command), (None, vec!["game".to_string(), "-a".to_string()]));
    }

    #[test]
    fn a_command_is_needed() {
        assert!(parse(&[]).is_err() && parse(&["--"]).is_err() && parse(&["--title"]).is_err());
    }

    #[test]
    fn a_process_lists_what_it_started() {
        let mut shell = Command::new("sh").args(["-c", "sleep 30 & wait"]).spawn().unwrap();
        thread::sleep(Duration::from_millis(300));
        let found = descendants(std::process::id());
        assert!(found.contains(&shell.id()) && found.len() >= 2, "{found:?}");
        quit::signal(&found, libc::SIGKILL);
        let _ = shell.wait();
    }
}
