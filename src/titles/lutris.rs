//! Lutris's games, from its game database.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use crate::config::Store;

use super::Found;

pub(super) fn scan(home: &Path) -> Vec<Found> {
    [home.join(".local/share/lutris/pga.db"), home.join(".var/app/net.lutris.Lutris/data/lutris/pga.db")]
        .iter()
        .filter(|db| db.is_file())
        .flat_map(|db| read(db).unwrap_or_default())
        .collect()
}

/// The installed games. Steam's own games Lutris lists are left to the Steam import.
fn read(db: &Path) -> rusqlite::Result<Vec<Found>> {
    let connection = Connection::open_with_flags(db, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    games(&connection)
}

fn games(connection: &Connection) -> rusqlite::Result<Vec<Found>> {
    let mut query = connection.prepare("SELECT id, name, runner, executable, directory FROM games WHERE installed = 1")?;
    let rows = query.query_map([], |row| {
        let text = |i| row.get::<_, Option<String>>(i).map(Option::unwrap_or_default);
        Ok((row.get::<_, i64>(0)?, text(1)?, text(2)?, text(3)?, text(4)?))
    })?;
    Ok(rows
        .flatten()
        .filter(|(_, name, runner, ..)| !name.trim().is_empty() && runner != "steam")
        .map(|(id, name, _, executable, directory)| Found {
            store: Store::Lutris,
            store_id: id.to_string(),
            runner: String::new(),
            name: name.trim().to_string(),
            steam: None,
            folder: (!directory.is_empty()).then(|| PathBuf::from(directory)),
            programs: (!executable.is_empty()).then_some(executable).into_iter().collect(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_installed_games() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE games (id INTEGER PRIMARY KEY, name TEXT, runner TEXT, executable TEXT, directory TEXT, installed INTEGER);
             INSERT INTO games VALUES (1, 'Total War: Rome II ', 'wine', '/g/Launcher.exe', NULL, 1);
             INSERT INTO games VALUES (2, 'Not here', 'linux', NULL, NULL, 0);
             INSERT INTO games VALUES (3, 'On Steam', 'steam', NULL, NULL, 1);",
        )
        .unwrap();
        let found = games(&db).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].store_id.as_str(), found[0].name.as_str()), ("1", "Total War: Rome II"));
        assert_eq!(found[0].programs, ["/g/Launcher.exe"]);
    }
}
