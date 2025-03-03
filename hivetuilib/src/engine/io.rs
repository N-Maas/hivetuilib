use std::{
    fmt::Display,
    fs::File,
    io::{self, BufRead, BufWriter, Write},
    path::Path,
};

use crate::{engine::INTERNAL_ERROR, GameData};

use super::{GameState, LoggingEngine};

const PLAYER_SEPARATOR: char = 'P';
const CURRENT_STATE: &str = "C";

#[derive(Debug)]
#[non_exhaustive]
pub enum CompatibilityPolicy {
    MajorLessEqual,
    MajorEqual,
    MinorLessEqual,
    MinorEqual,
}

impl CompatibilityPolicy {
    fn is_compatible(&self, game: [u32; 2], save: [u32; 2]) -> bool {
        match self {
            CompatibilityPolicy::MajorLessEqual => save[0] <= game[0],
            CompatibilityPolicy::MajorEqual => save[0] == game[0],
            CompatibilityPolicy::MinorLessEqual => save[0] == game[0] && save[1] <= game[1],
            CompatibilityPolicy::MinorEqual => save == game,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedLog {
    pub log: Vec<(usize, usize)>,
    /// Attention: redo information is stored in reverse order!
    pub redo_stack: Vec<(usize, usize)>,
}

/// Serialized string of the given initial game state, provided as key-value pairs.
pub fn serialize_initial_state<I>(initial_state: I) -> String
where
    I: IntoIterator<Item = (String, String)>,
{
    let key_val_pairs = initial_state
        .into_iter()
        .map(|(key, val)| {
            // TODO: escaping of spaces / newlines
            assert!(!key.contains(' ') && !key.contains('\n'));
            assert!(!val.contains(' ') && !val.contains('\n'));
            key + " " + &val
        })
        .collect::<Vec<_>>();
    key_val_pairs.join(" ")
}

/// Serialized string of the given initial game state, provided as key-value pairs.
pub fn deserialize_initial_state(input: &str) -> Result<Vec<(String, String)>, String> {
    let mut key_val_pairs = Vec::new();
    let mut it = input.split_ascii_whitespace().peekable();
    loop {
        match (it.next(), it.next()) {
            (Some(key), Some(val)) => key_val_pairs.push((key.to_string(), val.to_string())),
            (None, None) => break,
            (_, None) => return Err("Uneven number of key-value entries".to_string()),
            _ => unreachable!(),
        }
    }
    Ok(key_val_pairs)
}

/// Saves the game state to the given file. Initial game state can be provided as key-value pairs.
///
/// Note: Atomicity of file acccess needs to be ensured by the application.
pub fn save_game_to_file<H: AsRef<str>, I>(
    path: &Path,
    header: H,
    version: [u32; 2],
    initial_state: I,
    num_players: usize,
    log: SerializedLog,
) -> Result<(), io::Error>
where
    I: IntoIterator<Item = (String, String)>,
{
    let file = File::create(path)?;
    save_game(
        BufWriter::new(file),
        header,
        version,
        initial_state,
        num_players,
        log,
    )
}

/// Saves the game state via the provided writer. Initial game state can be provided as key-value pairs.
pub fn save_game<W: Write, H: AsRef<str>, I>(
    mut writer: W,
    header: H,
    version: [u32; 2],
    initial_state: I,
    num_players: usize,
    log: SerializedLog,
) -> Result<(), io::Error>
where
    I: IntoIterator<Item = (String, String)>,
{
    writeln!(writer, "{} v{}.{}", header.as_ref(), version[0], version[1])?;
    let initial_state = serialize_initial_state(initial_state);
    writeln!(writer, "{initial_state}")?;
    writeln!(writer, "{num_players}")?;
    for (index, player) in log.log.into_iter() {
        writeln!(writer, "{index}{PLAYER_SEPARATOR}{player}")?;
    }
    writeln!(writer, "{CURRENT_STATE}")?;
    for (index, player) in log.redo_stack.into_iter() {
        writeln!(writer, "{index}{PLAYER_SEPARATOR}{player}")?;
    }
    Ok(())
}

// TODO: Display, Error?
#[derive(Debug)]
pub enum LoadGameError {
    IO(io::Error),
    /// Syntactic error
    InvalidFileContent {
        line: usize,
        msg: String,
    },
    /// Version error
    VersionMismatch {
        game: [u32; 2],
        save: [u32; 2],
    },
    /// Semantic error: index is not valid for decision
    InvalidDecisionIndex {
        decision_nr: usize,
        index: usize,
        max_index: usize,
    },
    /// Semantic error: game state expects different player than provided
    UnexpectedPlayer {
        decision_nr: usize,
        player: usize,
        expected_player: usize,
    },
    /// Semantic error: decision provided, but game is over
    GameAlreadyFinished {
        decision_nr: usize,
    },
}

impl LoadGameError {
    fn from_file<S: ToString>(line: usize, msg: S) -> Self {
        Self::InvalidFileContent {
            line,
            msg: msg.to_string(),
        }
    }
}

impl From<io::Error> for LoadGameError {
    fn from(value: io::Error) -> Self {
        LoadGameError::IO(value)
    }
}

impl From<String> for LoadGameError {
    fn from(value: String) -> Self {
        LoadGameError::from_file(0, value)
    }
}

impl Display for LoadGameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadGameError::IO(e) => write!(f, "{e}"),
            LoadGameError::InvalidFileContent { line, msg } => write!(f, "{msg} at line {line}"),
            LoadGameError::VersionMismatch { game, save }
                => write!(f, "saved version (v{}.{}) is incompatible with game version (v{}.{})", save[0], save[1], game[0], game[1]),
            LoadGameError::InvalidDecisionIndex { decision_nr, index, max_index }
                => write!(f, "provided index {index} for decision is invalid (expected at most {max_index}) at decision number {decision_nr}"),
            LoadGameError::UnexpectedPlayer { decision_nr, player, expected_player }
                => write!(f, "expected player {expected_player}, but got player {player} at decision number {decision_nr}"),
            LoadGameError::GameAlreadyFinished { decision_nr }
                => write!(f, "game is already finished at decision number {decision_nr}"),
        }
    }
}

/// Reads the game state via the provided reader into key-value pairs for the initial state
/// and a serialized log.
pub fn parse_saved_game<R: BufRead, H: AsRef<str>>(
    mut reader: R,
    expected_header: H,
    version: [u32; 2],
    compatibility_policy: CompatibilityPolicy,
) -> Result<(Vec<(String, String)>, usize, SerializedLog), LoadGameError> {
    let mut curr_line = 0;
    let mut line = String::new();
    let mut next_line = |buf: &mut String, curr_line: &mut usize| -> Result<usize, LoadGameError> {
        buf.clear();
        let result = reader.read_line(buf)?;
        *curr_line += 1;
        if buf.ends_with('\n') {
            buf.pop();
        }
        Ok(result)
    };

    // read header and version
    next_line(&mut line, &mut curr_line)?;
    let expected = expected_header.as_ref();
    let header_err = || {
        LoadGameError::from_file(
            curr_line,
            format!("Invalid header: {line}, expected: {expected} vX.Y"),
        )
    };
    let version_str = line.split(' ').last().ok_or_else(header_err)?;
    let header_name = line.strip_suffix(version_str).unwrap().strip_suffix(' ');
    let version_str = version_str.strip_prefix('v').ok_or_else(header_err)?;
    let header_name = header_name.ok_or_else(header_err)?;
    if header_name != expected_header.as_ref() {
        return Err(header_err());
    }
    let save_version = version_str
        .split('.')
        .map(str::parse)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| header_err())?;
    let save_version: [u32; 2] = save_version.try_into().map_err(|_| header_err())?;
    if !compatibility_policy.is_compatible(version, save_version) {
        return Err(LoadGameError::VersionMismatch {
            game: version,
            save: save_version,
        });
    }

    // read initial state
    next_line(&mut line, &mut curr_line)?;
    let initial_state =
        deserialize_initial_state(&line).map_err(|msg| LoadGameError::from_file(curr_line, msg))?;

    // read number of players
    next_line(&mut line, &mut curr_line)?;
    let num_players = line
        .parse::<usize>()
        .map_err(|_| LoadGameError::from_file(curr_line, "Expected player number"))?;

    // read decisions
    let mut result = SerializedLog {
        log: Vec::new(),
        redo_stack: Vec::new(),
    };
    let mut found_current = false;
    while next_line(&mut line, &mut curr_line)? > 0 {
        if line == CURRENT_STATE && found_current {
            return Err(LoadGameError::from_file(
                curr_line,
                "Current state is ambiguous",
            ));
        } else if line == CURRENT_STATE {
            found_current = true;
            continue;
        }
        let mut split = line.split(PLAYER_SEPARATOR);
        if let (Some(index), Some(player), None) = (split.next(), split.next(), split.next()) {
            let index = index
                .parse::<usize>()
                .map_err(|e| LoadGameError::from_file(curr_line, format!("Invalid number: {e}")))?;
            let player = player
                .parse::<usize>()
                .map_err(|e| LoadGameError::from_file(curr_line, format!("Invalid number: {e}")))?;
            if found_current {
                result.redo_stack.push((index, player));
            } else {
                result.log.push((index, player));
            }
        } else {
            return Err(LoadGameError::from_file(
                curr_line,
                format!("Invalid line: {line}, expected: <dec>{PLAYER_SEPARATOR}<player>"),
            ));
        }
    }
    Ok((initial_state, num_players, result))
}

/// Loads the game state via a function providing the initial state
pub fn restore_game_state<T: GameData, F>(
    num_players: usize,
    create_data: F,
    log: SerializedLog,
) -> Result<LoggingEngine<T>, LoadGameError>
where
    F: Fn() -> Result<T, String>,
{
    let mut result = restore_game_state_impl(num_players, create_data()?, log.log.iter())?;
    if !log.redo_stack.is_empty() {
        // apply full log to verify correctness of redo stack
        restore_game_state_impl(
            num_players,
            create_data()?,
            log.log.iter().chain(log.redo_stack.iter().rev()),
        )?;
        result.log_mut().redo_stack = log.redo_stack;
    }
    Ok(result)
}

pub fn restore_game_state_impl<'a, T: GameData>(
    num_players: usize,
    data: T,
    log: impl Iterator<Item = &'a (usize, usize)>,
) -> Result<LoggingEngine<T>, LoadGameError> {
    let mut engine = LoggingEngine::new_logging(num_players, data);
    for (i, &(index, player)) in log.enumerate() {
        let decision = engine
            .get_decision()
            .ok_or(LoadGameError::GameAlreadyFinished { decision_nr: i })?;
        if index >= decision.option_count() {
            return Err(LoadGameError::InvalidDecisionIndex {
                decision_nr: i,
                index,
                max_index: decision.option_count(),
            });
        } else if player != decision.player() {
            return Err(LoadGameError::UnexpectedPlayer {
                decision_nr: i,
                player,
                expected_player: decision.player(),
            });
        }
        match engine.pull() {
            GameState::PendingDecision(d) => d.apply_option(index),
            _ => panic!("{}", INTERNAL_ERROR),
        };
    }
    Ok(engine)
}

/// Loads the game state via the provided reader and a function to interpret the key-value pairs
/// representing the initial state.
pub fn load_game<T: GameData, R: BufRead, H: AsRef<str>, F>(
    reader: R,
    expected_header: H,
    version: [u32; 2],
    compatibility_policy: CompatibilityPolicy,
    parse_initial_state: F,
) -> Result<LoggingEngine<T>, LoadGameError>
where
    F: Fn(&[(String, String)]) -> Result<T, String>,
{
    let (initial_state, num_players, log) =
        parse_saved_game(reader, expected_header, version, compatibility_policy)?;
    restore_game_state(num_players, || parse_initial_state(&initial_state), log)
}
