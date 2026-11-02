//! `padwight debug install-pack`: puts a setup pack into the config folder the way importing it
//! in the settings window does, and makes its profile the active one. For test runs (scenario
//! tests start from a pack of the library) on a config folder of their own.

use anyhow::{Context, Result, bail};

use crate::{
    config::{Config, ProfileRef},
    pack,
};


pub fn run(args: &[&str]) -> Result<()> {
    let (file, profile) = match args {
        [file] => (file, None),
        [file, profile] => (file, Some(*profile)),
        _ => bail!("usage: padwight debug install-pack <file> [profile]"),
    };
    let text = std::fs::read_to_string(file).with_context(|| format!("cannot read {file}"))?;
    let parsed = pack::parse(&text).with_context(|| format!("{file} is not a setup pack"))?;
    let mut config = Config::load()?;
    let plan = pack::plan(&config, parsed, false);
    let name = pack::apply(&mut config, &plan, &pack::Choices::default());
    let game = config.games.iter().find(|g| g.name == name).context("the pack did not import")?;
    let profile = match profile {
        Some(p) => game.profile(p).map(|p| p.name.clone()).with_context(|| format!("{name} has no profile {p:?}"))?,
        None => game.profiles.first().map(|p| p.name.clone()).with_context(|| format!("{name} has no profiles"))?,
    };
    config.active = ProfileRef::new(Some(&name), &profile);
    config.save()
}
