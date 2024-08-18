use std::{
    fs::File,
    io::{self, BufWriter, Write},
    path::Path,
};

const PLAYER_SEPARATOR: char = 'P';
const CURRENT_STATE: char = 'C';

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedLog {
    pub log: Vec<(usize, usize)>,
    pub redo_stack: Vec<(usize, usize)>,
}

/// Saves the game state to the given file. Initial game state can be provided as key-value pairs.
///
/// Note: Atomicity of file acccess needs to be ensured by the application.
pub fn save_game_to_file<H: AsRef<str>, I>(
    path: &Path,
    header: H,
    initial_state: I,
    log: SerializedLog,
) -> Result<(), io::Error>
where
    I: IntoIterator<Item = (String, String)>,
{
    let file = File::create(path)?;
    save_game(BufWriter::new(file), header, initial_state, log)
}

/// Saves the game state via the provided writer. Initial game state can be provided as key-value pairs.
pub fn save_game<W: Write, H: AsRef<str>, I>(
    mut writer: W,
    header: H,
    initial_state: I,
    log: SerializedLog,
) -> Result<(), io::Error>
where
    I: IntoIterator<Item = (String, String)>,
{
    writeln!(writer, "{}", header.as_ref())?;
    let key_val_pairs = initial_state
        .into_iter()
        .map(|(key, val)| {
            // TODO: escaping of spaces / newlines
            assert!(!key.contains(" ") && !key.contains("\n"));
            assert!(!val.contains(" ") && !val.contains("\n"));
            key + " " + &val
        })
        .collect::<Vec<_>>();
    writeln!(writer, "{}", key_val_pairs.join(" "))?;
    for (index, player) in log.log.into_iter() {
        writeln!(writer, "{index}{PLAYER_SEPARATOR}{player}")?;
    }
    writeln!(writer, "{CURRENT_STATE}")?;
    for (index, player) in log.redo_stack.into_iter() {
        writeln!(writer, "{index}{PLAYER_SEPARATOR}{player}")?;
    }
    Ok(())
}
