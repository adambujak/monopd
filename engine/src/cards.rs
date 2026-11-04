//! Card definitions for the 106-card Monopoly Deal deck.
//!
//! Every physical card has a `CardId` (0..106) that indexes into [`DECK`].
//! What matters for play is the card's [`Kind`]: two cards of the same kind
//! are fully interchangeable, so the engine's actions and observations are
//! expressed in kinds, never in card ids.

use std::fmt;

pub type CardId = u8;

pub const NUM_COLORS: usize = 10;
/// Number of distinct card kinds (see [`Kind::index`]).
pub const NUM_KINDS: usize = 40;
/// Kinds that can sit on the table as properties (10 plain colours + 8 wilds).
pub const NUM_PROPERTY_KINDS: usize = 18;
pub const DECK_SIZE: usize = 106;

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Color {
    Brown,
    LightBlue,
    Pink,
    Orange,
    Red,
    Yellow,
    Green,
    DarkBlue,
    Railroad,
    Utility,
}

pub const COLORS: [Color; NUM_COLORS] = [
    Color::Brown,
    Color::LightBlue,
    Color::Pink,
    Color::Orange,
    Color::Red,
    Color::Yellow,
    Color::Green,
    Color::DarkBlue,
    Color::Railroad,
    Color::Utility,
];

/// Rent charged for 1, 2, ... properties of a colour. Length = set size.
const RENT_LADDER: [&[u8]; NUM_COLORS] = [
    &[1, 2],       // Brown
    &[1, 2, 3],    // LightBlue
    &[1, 2, 4],    // Pink
    &[1, 3, 5],    // Orange
    &[2, 3, 6],    // Red
    &[2, 4, 6],    // Yellow
    &[2, 4, 7],    // Green
    &[3, 8],       // DarkBlue
    &[1, 2, 3, 4], // Railroad
    &[1, 2],       // Utility
];

const PROPERTY_VALUE: [u8; NUM_COLORS] = [1, 1, 2, 2, 3, 3, 4, 4, 2, 2];

pub const HOUSE_RENT: u8 = 3;
pub const HOTEL_RENT: u8 = 4;

impl Color {
    pub const fn idx(self) -> usize {
        self as usize
    }
    pub fn from_idx(i: usize) -> Color {
        COLORS[i]
    }
    /// Number of properties needed for a complete set.
    pub const fn set_size(self) -> usize {
        RENT_LADDER[self as usize].len()
    }
    /// Base rent for `count` properties of this colour (capped at a full set).
    pub fn rent(self, count: usize) -> u8 {
        if count == 0 {
            return 0;
        }
        let ladder = RENT_LADDER[self.idx()];
        ladder[count.min(ladder.len()) - 1]
    }
    pub const fn property_value(self) -> u8 {
        PROPERTY_VALUE[self as usize]
    }
    /// Houses and hotels can't go on railroads or utilities.
    pub const fn allows_buildings(self) -> bool {
        !matches!(self, Color::Railroad | Color::Utility)
    }
    pub const fn bit(self) -> u16 {
        1 << (self as u16)
    }
    pub fn name(self) -> &'static str {
        match self {
            Color::Brown => "Brown",
            Color::LightBlue => "LightBlue",
            Color::Pink => "Pink",
            Color::Orange => "Orange",
            Color::Red => "Red",
            Color::Yellow => "Yellow",
            Color::Green => "Green",
            Color::DarkBlue => "DarkBlue",
            Color::Railroad => "Railroad",
            Color::Utility => "Utility",
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

pub const ALL_COLORS_MASK: u16 = (1 << NUM_COLORS) - 1;

/// Iterate the colours present in a bitmask.
pub fn iter_colors(mask: u16) -> impl Iterator<Item = Color> {
    COLORS.into_iter().filter(move |c| mask & c.bit() != 0)
}

// ---------------------------------------------------------------------------
// Card sub-types
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum WildType {
    DarkBlueGreen,
    LightBlueBrown,
    PinkOrange,
    GreenRailroad,
    LightBlueRailroad,
    UtilityRailroad,
    RedYellow,
    /// The multi-colour wild: any colour, no cash value.
    Rainbow,
}

pub const WILD_TYPES: [WildType; 8] = [
    WildType::DarkBlueGreen,
    WildType::LightBlueBrown,
    WildType::PinkOrange,
    WildType::GreenRailroad,
    WildType::LightBlueRailroad,
    WildType::UtilityRailroad,
    WildType::RedYellow,
    WildType::Rainbow,
];

impl WildType {
    pub const fn colors(self) -> u16 {
        use Color::*;
        match self {
            WildType::DarkBlueGreen => DarkBlue.bit() | Green.bit(),
            WildType::LightBlueBrown => LightBlue.bit() | Brown.bit(),
            WildType::PinkOrange => Pink.bit() | Orange.bit(),
            WildType::GreenRailroad => Green.bit() | Railroad.bit(),
            WildType::LightBlueRailroad => LightBlue.bit() | Railroad.bit(),
            WildType::UtilityRailroad => Utility.bit() | Railroad.bit(),
            WildType::RedYellow => Red.bit() | Yellow.bit(),
            WildType::Rainbow => ALL_COLORS_MASK,
        }
    }
    pub const fn value(self) -> u8 {
        match self {
            WildType::DarkBlueGreen => 4,
            WildType::LightBlueBrown => 1,
            WildType::PinkOrange => 2,
            WildType::GreenRailroad => 4,
            WildType::LightBlueRailroad => 4,
            WildType::UtilityRailroad => 2,
            WildType::RedYellow => 3,
            WildType::Rainbow => 0,
        }
    }
    const fn count(self) -> usize {
        match self {
            WildType::PinkOrange | WildType::RedYellow | WildType::Rainbow => 2,
            _ => 1,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ActionType {
    DealBreaker,
    JustSayNo,
    SlyDeal,
    ForcedDeal,
    DebtCollector,
    Birthday,
    DoubleRent,
    House,
    Hotel,
    PassGo,
}

pub const NUM_ACTION_TYPES: usize = 10;
pub const ACTION_TYPES: [ActionType; NUM_ACTION_TYPES] = [
    ActionType::DealBreaker,
    ActionType::JustSayNo,
    ActionType::SlyDeal,
    ActionType::ForcedDeal,
    ActionType::DebtCollector,
    ActionType::Birthday,
    ActionType::DoubleRent,
    ActionType::House,
    ActionType::Hotel,
    ActionType::PassGo,
];

impl ActionType {
    pub const fn value(self) -> u8 {
        match self {
            ActionType::DealBreaker => 5,
            ActionType::JustSayNo => 4,
            ActionType::SlyDeal => 3,
            ActionType::ForcedDeal => 3,
            ActionType::DebtCollector => 3,
            ActionType::Birthday => 2,
            ActionType::DoubleRent => 1,
            ActionType::House => 3,
            ActionType::Hotel => 4,
            ActionType::PassGo => 1,
        }
    }
    const fn count(self) -> usize {
        match self {
            ActionType::DealBreaker | ActionType::DoubleRent | ActionType::Hotel => 2,
            ActionType::PassGo => 10,
            _ => 3,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            ActionType::DealBreaker => "Deal Breaker",
            ActionType::JustSayNo => "Just Say No",
            ActionType::SlyDeal => "Sly Deal",
            ActionType::ForcedDeal => "Forced Deal",
            ActionType::DebtCollector => "Debt Collector",
            ActionType::Birthday => "It's My Birthday",
            ActionType::DoubleRent => "Double The Rent",
            ActionType::House => "House",
            ActionType::Hotel => "Hotel",
            ActionType::PassGo => "Pass Go",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum RentType {
    DarkBlueGreen,
    RedYellow,
    PinkOrange,
    LightBlueBrown,
    RailroadUtility,
    /// Any colour, but charges a single chosen player.
    Wild,
}

pub const RENT_TYPES: [RentType; 6] = [
    RentType::DarkBlueGreen,
    RentType::RedYellow,
    RentType::PinkOrange,
    RentType::LightBlueBrown,
    RentType::RailroadUtility,
    RentType::Wild,
];

impl RentType {
    pub const fn colors(self) -> u16 {
        use Color::*;
        match self {
            RentType::DarkBlueGreen => DarkBlue.bit() | Green.bit(),
            RentType::RedYellow => Red.bit() | Yellow.bit(),
            RentType::PinkOrange => Pink.bit() | Orange.bit(),
            RentType::LightBlueBrown => LightBlue.bit() | Brown.bit(),
            RentType::RailroadUtility => Railroad.bit() | Utility.bit(),
            RentType::Wild => ALL_COLORS_MASK,
        }
    }
    pub const fn value(self) -> u8 {
        match self {
            RentType::Wild => 3,
            _ => 1,
        }
    }
    const fn count(self) -> usize {
        match self {
            RentType::Wild => 3,
            _ => 2,
        }
    }
    /// The (unique) two-colour rent card that covers `color`.
    pub const fn for_color(color: Color) -> RentType {
        use Color::*;
        match color {
            DarkBlue | Green => RentType::DarkBlueGreen,
            Red | Yellow => RentType::RedYellow,
            Pink | Orange => RentType::PinkOrange,
            LightBlue | Brown => RentType::LightBlueBrown,
            Railroad | Utility => RentType::RailroadUtility,
        }
    }
}

pub const MONEY_VALUES: [u8; 6] = [1, 2, 3, 4, 5, 10];
const MONEY_COUNTS: [usize; 6] = [6, 5, 3, 3, 2, 1];

// ---------------------------------------------------------------------------
// Kind: the canonical identity of a card
// ---------------------------------------------------------------------------

/// What a card *is*. Cards of equal kind are interchangeable.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Property(Color),
    Wild(WildType),
    Action(ActionType),
    Rent(RentType),
    /// Index into [`MONEY_VALUES`].
    Money(u8),
}

impl Kind {
    /// Dense index 0..40: properties 0-9, wilds 10-17, actions 18-27,
    /// rents 28-33, money 34-39. Property kinds (0..18) come first so they
    /// can index property slots directly.
    pub const fn index(self) -> usize {
        match self {
            Kind::Property(c) => c as usize,
            Kind::Wild(w) => 10 + w as usize,
            Kind::Action(a) => 18 + a as usize,
            Kind::Rent(r) => 28 + r as usize,
            Kind::Money(m) => 34 + m as usize,
        }
    }

    pub fn from_index(i: usize) -> Kind {
        match i {
            0..=9 => Kind::Property(COLORS[i]),
            10..=17 => Kind::Wild(WILD_TYPES[i - 10]),
            18..=27 => Kind::Action(ACTION_TYPES[i - 18]),
            28..=33 => Kind::Rent(RENT_TYPES[i - 28]),
            34..=39 => Kind::Money((i - 34) as u8),
            _ => panic!("kind index out of range: {i}"),
        }
    }

    /// Cash value when banked or paid.
    pub const fn value(self) -> u8 {
        match self {
            Kind::Property(c) => c.property_value(),
            Kind::Wild(w) => w.value(),
            Kind::Action(a) => a.value(),
            Kind::Rent(r) => r.value(),
            Kind::Money(m) => MONEY_VALUES[m as usize],
        }
    }

    /// Colours this card may be placed as (0 for non-properties).
    pub const fn colors(self) -> u16 {
        match self {
            Kind::Property(c) => c.bit(),
            Kind::Wild(w) => w.colors(),
            _ => 0,
        }
    }

    pub const fn is_property(self) -> bool {
        matches!(self, Kind::Property(_) | Kind::Wild(_))
    }

    pub const fn is_wild(self) -> bool {
        matches!(self, Kind::Wild(_))
    }

    const fn count(self) -> usize {
        match self {
            Kind::Property(c) => c.set_size(),
            Kind::Wild(w) => w.count(),
            Kind::Action(a) => a.count(),
            Kind::Rent(r) => r.count(),
            Kind::Money(m) => MONEY_COUNTS[m as usize],
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Property(c) => write!(f, "{c} property"),
            Kind::Wild(WildType::Rainbow) => write!(f, "Rainbow wild"),
            Kind::Wild(w) => {
                let cs: Vec<_> = iter_colors(w.colors()).map(|c| c.name()).collect();
                write!(f, "{} wild (${}M)", cs.join("/"), w.value())
            }
            Kind::Action(a) => write!(f, "{}", a.name()),
            Kind::Rent(RentType::Wild) => write!(f, "Wild rent"),
            Kind::Rent(r) => {
                let cs: Vec<_> = iter_colors(r.colors()).map(|c| c.name()).collect();
                write!(f, "{} rent", cs.join("/"))
            }
            Kind::Money(m) => write!(f, "${}M", MONEY_VALUES[*m as usize]),
        }
    }
}

// ---------------------------------------------------------------------------
// The deck
// ---------------------------------------------------------------------------

const fn all_kinds() -> [Kind; NUM_KINDS] {
    let mut out = [Kind::Money(0); NUM_KINDS];
    let mut i = 0;
    while i < NUM_COLORS {
        out[i] = Kind::Property(COLORS[i]);
        i += 1;
    }
    let mut w = 0;
    while w < 8 {
        out[10 + w] = Kind::Wild(WILD_TYPES[w]);
        w += 1;
    }
    let mut a = 0;
    while a < NUM_ACTION_TYPES {
        out[18 + a] = Kind::Action(ACTION_TYPES[a]);
        a += 1;
    }
    let mut r = 0;
    while r < 6 {
        out[28 + r] = Kind::Rent(RENT_TYPES[r]);
        r += 1;
    }
    let mut m = 0;
    while m < 6 {
        out[34 + m] = Kind::Money(m as u8);
        m += 1;
    }
    out
}

pub const ALL_KINDS: [Kind; NUM_KINDS] = all_kinds();

const fn build_deck() -> [Kind; DECK_SIZE] {
    let mut deck = [Kind::Money(0); DECK_SIZE];
    let mut n = 0;
    let mut k = 0;
    while k < NUM_KINDS {
        let kind = ALL_KINDS[k];
        let mut c = 0;
        while c < kind.count() {
            deck[n] = kind;
            n += 1;
            c += 1;
        }
        k += 1;
    }
    assert!(n == DECK_SIZE, "deck must have 106 cards");
    deck
}

const fn kind_counts() -> [u8; NUM_KINDS] {
    let mut out = [0u8; NUM_KINDS];
    let mut k = 0;
    while k < NUM_KINDS {
        out[k] = ALL_KINDS[k].count() as u8;
        k += 1;
    }
    out
}

/// Kind of every card id.
pub static DECK: [Kind; DECK_SIZE] = build_deck();
/// How many copies of each kind exist in the deck.
pub const KIND_COUNTS: [u8; NUM_KINDS] = kind_counts();

#[inline]
pub fn kind_of(card: CardId) -> Kind {
    DECK[card as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(f: impl Fn(&Kind) -> bool) -> usize {
        DECK.iter().filter(|k| f(k)).count()
    }

    #[test]
    fn deck_has_the_official_106_cards() {
        assert_eq!(DECK.len(), 106);
        assert_eq!(count(|k| matches!(k, Kind::Property(_))), 28);
        assert_eq!(count(|k| matches!(k, Kind::Wild(_))), 11);
        assert_eq!(count(|k| matches!(k, Kind::Action(_))), 34);
        assert_eq!(count(|k| matches!(k, Kind::Rent(_))), 13);
        assert_eq!(count(|k| matches!(k, Kind::Money(_))), 20);
        let money: u32 = DECK.iter().filter(|k| matches!(k, Kind::Money(_))).map(|k| k.value() as u32).sum();
        assert_eq!(money, 57);
        assert_eq!(KIND_COUNTS.iter().map(|&n| n as usize).sum::<usize>(), DECK_SIZE);
    }

    #[test]
    fn card_counts_and_values_match_the_real_deck() {
        let n = |k: Kind| KIND_COUNTS[k.index()];
        assert_eq!(n(Kind::Action(ActionType::ForcedDeal)), 3);
        assert_eq!(n(Kind::Action(ActionType::Hotel)), 2);
        assert_eq!(n(Kind::Action(ActionType::PassGo)), 10);
        assert_eq!(n(Kind::Rent(RentType::Wild)), 3);
        assert_eq!(n(Kind::Wild(WildType::Rainbow)), 2);
        assert_eq!(Kind::Wild(WildType::UtilityRailroad).value(), 2);
        assert_eq!(Kind::Wild(WildType::Rainbow).value(), 0);
        assert_eq!(Kind::Action(ActionType::DealBreaker).value(), 5);
        assert_eq!(Kind::Rent(RentType::Wild).value(), 3);
        // One plain property card per slot in a set.
        for c in COLORS {
            assert_eq!(n(Kind::Property(c)) as usize, c.set_size());
        }
    }

    #[test]
    fn kind_index_is_dense_and_round_trips() {
        for (i, &k) in ALL_KINDS.iter().enumerate() {
            assert_eq!(k.index(), i);
            assert_eq!(Kind::from_index(i), k);
        }
        // Property kinds come first, so they can index property slots.
        assert!(ALL_KINDS[..NUM_PROPERTY_KINDS].iter().all(|k| k.is_property()));
        assert!(ALL_KINDS[NUM_PROPERTY_KINDS..].iter().all(|k| !k.is_property()));
    }

    #[test]
    fn rent_ladder_caps_at_a_full_set() {
        assert_eq!(Color::Brown.rent(0), 0);
        assert_eq!(Color::Brown.rent(1), 1);
        assert_eq!(Color::Brown.rent(2), 2);
        assert_eq!(Color::Brown.rent(5), 2);
        assert_eq!(Color::DarkBlue.rent(2), 8);
        assert_eq!(Color::Railroad.rent(4), 4);
        assert_eq!(Color::Green.rent(3), 7);
    }

    #[test]
    fn every_colour_has_exactly_one_two_colour_rent_card() {
        for c in COLORS {
            let covering: Vec<_> = RENT_TYPES[..5].iter().filter(|r| r.colors() & c.bit() != 0).collect();
            assert_eq!(covering, vec![&RentType::for_color(c)], "{c}");
        }
    }

    #[test]
    fn wilds_cover_two_colours_except_the_rainbow() {
        for w in WILD_TYPES {
            let n = iter_colors(w.colors()).count();
            assert_eq!(n, if w == WildType::Rainbow { NUM_COLORS } else { 2 }, "{w:?}");
        }
    }

    #[test]
    fn buildings_are_not_allowed_on_railroads_or_utilities() {
        let no_buildings: Vec<_> = COLORS.into_iter().filter(|c| !c.allows_buildings()).collect();
        assert_eq!(no_buildings, vec![Color::Railroad, Color::Utility]);
    }
}
