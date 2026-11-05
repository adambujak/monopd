"""Monopoly Deal engine (Rust) for Python.

    import monopd
    g = monopd.Game(num_players=2, seed=0)
    while not g.is_over():
        seat = g.current_player()
        obs = g.observe(seat)          # list of OBS_SIZE floats
        choices = g.legal_actions()    # list of action indices
        g.step(choices[0])
    print(g.winner, g.returns())
"""

NUM_ACTIONS: int
"""Size of the action space; actions are ints in range(NUM_ACTIONS)."""
OBS_SIZE: int
"""Length of an observation vector."""
MAX_PLAYERS: int

class Game:
    def __init__(self, num_players: int = 2, seed: int = 0) -> None: ...
    def current_player(self) -> int | None:
        """Who must decide next (can be a non-active player, e.g. paying rent), or None when over."""
    def is_over(self) -> bool: ...
    @property
    def winner(self) -> int | None: ...
    @property
    def turn(self) -> int: ...
    @property
    def num_players(self) -> int: ...
    def legal_actions(self) -> list[int]:
        """Indices of the legal actions for the current player."""
    def legal_mask(self) -> list[bool]:
        """One bool per action index (length NUM_ACTIONS)."""
    def observe(self, seat: int) -> list[float]:
        """What `seat` can see, as OBS_SIZE floats."""
    def step(self, action: int) -> None:
        """Apply a legal action index. Raises ValueError if illegal."""
    def returns(self) -> list[float]:
        """+1 for the winner, -1 for others, 0 for everyone on a draw."""
    def describe(self, seat: int) -> str:
        """A readable view of the table from `seat`'s perspective."""
    def clone(self) -> Game: ...
    def determinize(self, seat: int, seed: int) -> Game:
        """A copy with everything `seat` can't see re-dealt at random."""

class RandomBot:
    def __init__(self, seed: int = 0) -> None: ...
    def choose(self, game: Game) -> int:
        """A random legal action index for the current player."""

def action_name(index: int) -> str:
    """e.g. action_name(i) -> 'Charge Red rent (everyone)'."""
