//! Python bindings: `import monopd`.
//!
//! A thin wrapper over the Rust engine. Actions are plain ints (indices in
//! `0..NUM_ACTIONS`) and observations are lists of floats, so they drop
//! straight into `torch.tensor(...)`. See `monopd.pyi` for the Python API.

use monopd_engine::bots::RandomBot as EngineBot;
use monopd_engine::{Action, Game as EngineGame, StepError, MAX_PLAYERS, NUM_ACTIONS, OBS_SIZE};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

#[pyclass(module = "monopd")]
#[derive(Clone)]
struct Game {
    inner: EngineGame,
}

impl Game {
    fn check_seat(&self, seat: usize) -> PyResult<()> {
        if seat < self.inner.num_players {
            Ok(())
        } else {
            Err(PyValueError::new_err(format!("seat {seat} out of range for {} players", self.inner.num_players)))
        }
    }
}

#[pymethods]
impl Game {
    #[new]
    #[pyo3(signature = (num_players = 2, seed = 0))]
    fn new(num_players: usize, seed: u64) -> PyResult<Self> {
        if !(2..=MAX_PLAYERS).contains(&num_players) {
            return Err(PyValueError::new_err("Monopoly Deal supports 2-5 players"));
        }
        Ok(Game { inner: EngineGame::new(num_players, seed) })
    }

    /// Who must decide next, or None when the game is over.
    fn current_player(&self) -> Option<usize> {
        self.inner.current_player()
    }

    fn is_over(&self) -> bool {
        self.inner.is_over()
    }

    #[getter]
    fn winner(&self) -> Option<usize> {
        self.inner.winner
    }

    #[getter]
    fn turn(&self) -> u32 {
        self.inner.turn
    }

    #[getter]
    fn num_players(&self) -> usize {
        self.inner.num_players
    }

    /// Indices of the legal actions for the current player.
    fn legal_actions(&self) -> Vec<usize> {
        self.inner.legal_actions().into_iter().map(Action::index).collect()
    }

    /// One bool per action index (length NUM_ACTIONS).
    fn legal_mask(&self) -> Vec<bool> {
        self.inner.legal_mask()
    }

    /// What `seat` can see, as OBS_SIZE floats.
    fn observe(&self, seat: usize) -> PyResult<Vec<f32>> {
        self.check_seat(seat)?;
        Ok(self.inner.observe(seat))
    }

    /// Apply a legal action by index. Raises ValueError if it isn't legal.
    fn step(&mut self, action: usize) -> PyResult<()> {
        self.inner.step_index(action).map_err(|e| match e {
            StepError::Illegal(a) => PyValueError::new_err(format!("illegal action {action} ({a})")),
            StepError::OutOfRange(_) => PyValueError::new_err(format!("action {action} out of range")),
        })
    }

    /// Final reward per seat: +1 winner, -1 others, 0 each on a draw.
    fn returns(&self) -> Vec<f32> {
        self.inner.returns()
    }

    /// A readable view of the table from `seat`'s perspective.
    fn describe(&self, seat: usize) -> PyResult<String> {
        self.check_seat(seat)?;
        Ok(self.inner.describe(seat))
    }

    /// An independent copy (for search or trying things out).
    fn clone(&self) -> Self {
        Clone::clone(self)
    }

    fn __copy__(&self) -> Self {
        Clone::clone(self)
    }

    /// A copy with everything `seat` can't see re-dealt at random.
    fn determinize(&self, seat: usize, seed: u64) -> PyResult<Self> {
        self.check_seat(seat)?;
        Ok(Game { inner: self.inner.determinize(seat, seed) })
    }

    fn __repr__(&self) -> String {
        let state = match self.inner.winner {
            Some(w) => format!("winner=P{w}"),
            None if self.inner.is_over() => "draw".to_string(),
            None => format!("to_act=P{}", self.inner.current_player().unwrap_or(0)),
        };
        format!("<monopd.Game players={} turn={} {state}>", self.inner.num_players, self.inner.turn)
    }
}

/// Picks uniformly among legal actions.
#[pyclass(module = "monopd")]
struct RandomBot {
    inner: EngineBot,
}

#[pymethods]
impl RandomBot {
    #[new]
    #[pyo3(signature = (seed = 0))]
    fn new(seed: u64) -> Self {
        RandomBot { inner: EngineBot::new(seed) }
    }

    /// A random legal action index for the game's current player.
    fn choose(&mut self, game: &Game) -> PyResult<usize> {
        if game.inner.is_over() {
            return Err(PyValueError::new_err("the game is over"));
        }
        Ok(self.inner.choose(&game.inner).index())
    }
}

/// A readable name for an action index, e.g. "Charge Red rent (everyone)".
#[pyfunction]
fn action_name(index: usize) -> PyResult<String> {
    Action::from_index(index)
        .map(|a| a.to_string())
        .ok_or_else(|| PyValueError::new_err(format!("action {index} out of range")))
}

#[pymodule]
fn monopd(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Game>()?;
    m.add_class::<RandomBot>()?;
    m.add_function(wrap_pyfunction!(action_name, m)?)?;
    m.add("NUM_ACTIONS", NUM_ACTIONS)?;
    m.add("OBS_SIZE", OBS_SIZE)?;
    m.add("MAX_PLAYERS", MAX_PLAYERS)?;
    Ok(())
}
