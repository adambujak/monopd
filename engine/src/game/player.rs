//! A player's cards: hand, bank and one property pile per colour.

use super::RAINBOW;
use crate::cards::*;

/// All properties of one colour on a player's table, plus buildings.
///
/// Each colour is a single pile. A pile with at least `set_size` cards, at
/// least one of which really is that colour, is one complete set (extra cards
/// are protected along with it).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pile {
    pub cards: Vec<CardId>,
    pub house: Option<CardId>,
    pub hotel: Option<CardId>,
}

impl Pile {
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Some card of this kind in the pile (they're interchangeable).
    pub fn find(&self, kind: Kind) -> Option<CardId> {
        self.cards.iter().copied().find(|&c| kind_of(c) == kind)
    }

    /// Does it hold a card that really is this colour? A rainbow wild has no
    /// colour of its own, so rainbows alone can't charge rent or make a set.
    pub fn has_real(&self) -> bool {
        self.cards.iter().any(|&c| kind_of(c) != RAINBOW)
    }

    pub fn is_complete(&self, color: Color) -> bool {
        self.len() >= color.set_size() && self.has_real()
    }

    /// Can Sly Deal / Forced Deal take from it? Anything not in a complete set.
    pub fn is_stealable(&self, color: Color) -> bool {
        !self.is_empty() && !self.is_complete(color)
    }

    /// Cash value of everything in the pile, buildings included.
    pub fn value(&self) -> u32 {
        self.cards.iter().chain(&self.house).chain(&self.hotel).map(|&c| kind_of(c).value() as u32).sum()
    }

    pub(crate) fn remove(&mut self, card: CardId) {
        let i = self.cards.iter().position(|&c| c == card).expect("card not in pile");
        self.cards.remove(i);
    }

    pub(crate) fn take_buildings(&mut self) -> [Option<CardId>; 2] {
        [self.house.take(), self.hotel.take()]
    }

    /// Merge a set taken with Deal Breaker into this pile. Returns buildings
    /// that don't fit (this pile already had one of that kind).
    pub(crate) fn absorb(&mut self, other: Pile) -> Vec<CardId> {
        self.cards.extend(other.cards);
        let mut extra = Vec::new();
        for (mine, theirs) in [(&mut self.house, other.house), (&mut self.hotel, other.hotel)] {
            match (mine.is_some(), theirs) {
                (false, t) => *mine = t,
                (true, Some(t)) => extra.push(t),
                (true, None) => {}
            }
        }
        extra
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Player {
    pub hand: Vec<CardId>,
    /// In deposit order: the last card is the most recent.
    pub bank: Vec<CardId>,
    pub table: [Pile; NUM_COLORS],
}

impl Player {
    pub fn pile(&self, color: Color) -> &Pile {
        &self.table[color.idx()]
    }

    pub(crate) fn pile_mut(&mut self, color: Color) -> &mut Pile {
        &mut self.table[color.idx()]
    }

    pub fn is_complete(&self, color: Color) -> bool {
        self.pile(color).is_complete(color)
    }

    pub fn has_real(&self, color: Color) -> bool {
        self.pile(color).has_real()
    }

    pub fn complete_sets(&self) -> usize {
        COLORS.iter().filter(|&&c| self.is_complete(c)).count()
    }

    pub fn has_complete_set(&self) -> bool {
        COLORS.iter().any(|&c| self.is_complete(c))
    }

    pub fn has_stealable(&self) -> bool {
        COLORS.iter().any(|&c| self.pile(c).is_stealable(c))
    }

    /// Can a house (or, with `hotel`, a hotel) go on this colour?
    pub fn can_build(&self, color: Color, hotel: bool) -> bool {
        let pile = self.pile(color);
        let slot_free = if hotel { pile.house.is_some() && pile.hotel.is_none() } else { pile.house.is_none() };
        color.allows_buildings() && self.is_complete(color) && slot_free
    }

    /// Rent this player would charge for `color`.
    pub fn rent(&self, color: Color) -> u8 {
        let pile = self.pile(color);
        if !pile.has_real() {
            return 0;
        }
        let mut rent = color.rent(pile.len());
        if self.is_complete(color) {
            rent += pile.house.map_or(0, |_| HOUSE_RENT) + pile.hotel.map_or(0, |_| HOTEL_RENT);
        }
        rent
    }

    pub fn bank_value(&self) -> u32 {
        self.bank.iter().map(|&c| kind_of(c).value() as u32).sum()
    }

    /// Total value this player could hand over as payment.
    pub fn payable_value(&self) -> u32 {
        self.bank_value() + self.table.iter().map(Pile::value).sum::<u32>()
    }

    pub fn holds(&self, kind: Kind) -> bool {
        self.hand.iter().any(|&c| kind_of(c) == kind)
    }

    pub(crate) fn take_from_hand(&mut self, kind: Kind) -> CardId {
        let i = self.hand.iter().position(|&c| kind_of(c) == kind).expect("card kind not in hand");
        self.hand.swap_remove(i)
    }

    /// The most recently banked card of this kind (keeps deposit order).
    pub(crate) fn take_from_bank(&mut self, kind: Kind) -> CardId {
        let i = self.bank.iter().rposition(|&c| kind_of(c) == kind).expect("card kind not in bank");
        self.bank.remove(i)
    }

    pub(crate) fn take_building(&mut self, color: Color, hotel: bool) -> CardId {
        let pile = self.pile_mut(color);
        if hotel { pile.hotel.take() } else { pile.house.take() }.expect("no such building")
    }

    /// Buildings on a set that is no longer complete go to the bank.
    pub(crate) fn fix_buildings(&mut self, color: Color) {
        if !self.is_complete(color) {
            let buildings = self.pile_mut(color).take_buildings();
            self.bank.extend(buildings.into_iter().flatten());
        }
    }
}

/// The distinct kinds among `cards`, in first-seen order. Cards of one kind
/// are interchangeable, so legal actions list each kind once.
pub(crate) fn distinct(cards: &[CardId]) -> impl Iterator<Item = Kind> + '_ {
    let mut seen = 0u64;
    cards.iter().map(|&c| kind_of(c)).filter(move |k| {
        let bit = 1u64 << k.index();
        let fresh = seen & bit == 0;
        seen |= bit;
        fresh
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Kind = Kind::Property(Color::Red);
    const BROWN: Kind = Kind::Property(Color::Brown);

    /// Card ids for kinds, taken from the deck in order.
    fn ids(kinds: &[Kind]) -> Vec<CardId> {
        let mut used = [false; DECK_SIZE];
        kinds
            .iter()
            .map(|&k| {
                let i = (0..DECK_SIZE).find(|&i| !used[i] && DECK[i] == k).expect("not enough cards");
                used[i] = true;
                i as CardId
            })
            .collect()
    }

    fn pile(kinds: &[Kind]) -> Pile {
        Pile { cards: ids(kinds), ..Pile::default() }
    }

    fn building(hotel: bool) -> CardId {
        ids(&[Kind::Action(if hotel { ActionType::Hotel } else { ActionType::House })])[0]
    }

    #[test]
    fn pile_completeness_needs_size_and_a_real_card() {
        assert!(!pile(&[RED, RED]).is_complete(Color::Red));
        assert!(pile(&[RED, RED, RED]).is_complete(Color::Red));
        assert!(pile(&[RED, RAINBOW, RAINBOW]).is_complete(Color::Red));
        assert!(!pile(&[RAINBOW, RAINBOW]).is_complete(Color::Brown), "rainbows alone have no colour");
        assert!(pile(&[RED, RED, RED, RAINBOW]).is_complete(Color::Red), "overflow still complete");
    }

    #[test]
    fn only_incomplete_non_empty_piles_are_stealable() {
        assert!(!Pile::default().is_stealable(Color::Red));
        assert!(pile(&[RED]).is_stealable(Color::Red));
        assert!(!pile(&[BROWN, BROWN]).is_stealable(Color::Brown));
        assert!(pile(&[RAINBOW, RAINBOW]).is_stealable(Color::Brown));
    }

    #[test]
    fn rent_follows_the_ladder_and_adds_buildings() {
        let mut p = Player::default();
        assert_eq!(p.rent(Color::Red), 0);
        p.table[Color::Red.idx()] = pile(&[RED]);
        assert_eq!(p.rent(Color::Red), 2);
        p.table[Color::Red.idx()] = pile(&[RED, RED, RED, RAINBOW]);
        assert_eq!(p.rent(Color::Red), 6, "capped at a full set");
        p.pile_mut(Color::Red).house = Some(building(false));
        p.pile_mut(Color::Red).hotel = Some(building(true));
        assert_eq!(p.rent(Color::Red), 6 + 3 + 4);
        p.table[Color::Red.idx()] = pile(&[RAINBOW]);
        assert_eq!(p.rent(Color::Red), 0, "a lone rainbow charges nothing");
    }

    #[test]
    fn payable_value_counts_bank_properties_and_buildings() {
        let mut p = Player { bank: ids(&[Kind::Money(4)]), ..Player::default() }; // $5M
        p.table[Color::Brown.idx()] = pile(&[BROWN, BROWN, RAINBOW]); // 1 + 1 + 0
        p.pile_mut(Color::Brown).house = Some(building(false)); // 3
        assert_eq!(p.payable_value(), 5 + 2 + 3);
    }

    #[test]
    fn can_build_requires_complete_set_house_before_hotel_and_no_railroads() {
        let mut p = Player::default();
        p.table[Color::Red.idx()] = pile(&[RED, RED]);
        assert!(!p.can_build(Color::Red, false));
        p.pile_mut(Color::Red).cards.extend(ids(&[RED, RED, RED])[2..].iter());
        assert!(p.can_build(Color::Red, false));
        assert!(!p.can_build(Color::Red, true), "hotel needs a house");
        p.pile_mut(Color::Red).house = Some(building(false));
        assert!(!p.can_build(Color::Red, false));
        assert!(p.can_build(Color::Red, true));
        let rr = Kind::Property(Color::Railroad);
        p.table[Color::Railroad.idx()] = pile(&[rr, rr, rr, rr]);
        assert!(!p.can_build(Color::Railroad, false));
    }

    #[test]
    fn take_from_bank_takes_the_newest_and_keeps_order() {
        let cards = ids(&[Kind::Money(0), Kind::Money(1), Kind::Money(0)]);
        let mut p = Player { bank: cards.clone(), ..Player::default() };
        assert_eq!(p.take_from_bank(Kind::Money(0)), cards[2]);
        assert_eq!(p.bank, vec![cards[0], cards[1]]);
    }

    #[test]
    fn fix_buildings_moves_them_to_the_bank_when_the_set_breaks() {
        let mut p = Player::default();
        p.table[Color::Brown.idx()] = pile(&[BROWN, BROWN]);
        let house = building(false);
        p.pile_mut(Color::Brown).house = Some(house);
        p.fix_buildings(Color::Brown);
        assert_eq!(p.pile(Color::Brown).house, Some(house), "still complete");
        let c = p.pile(Color::Brown).cards[0];
        p.pile_mut(Color::Brown).remove(c);
        p.fix_buildings(Color::Brown);
        assert_eq!(p.pile(Color::Brown).house, None);
        assert_eq!(p.bank, vec![house]);
    }

    #[test]
    fn absorb_merges_cards_and_returns_buildings_that_dont_fit() {
        let houses = ids(&[Kind::Action(ActionType::House), Kind::Action(ActionType::House)]);
        let mut mine = pile(&[RED]);
        mine.house = Some(houses[0]);
        let mut theirs = Pile { cards: ids(&[RED, RED, RED])[1..].to_vec(), ..Pile::default() };
        theirs.house = Some(houses[1]);
        let extra = mine.absorb(theirs);
        assert_eq!(mine.len(), 3);
        assert_eq!(mine.house, Some(houses[0]));
        assert_eq!(extra, vec![houses[1]]);
    }

    #[test]
    fn distinct_lists_each_kind_once_in_first_seen_order() {
        let cards = ids(&[RED, BROWN, RED, Kind::Money(0), BROWN]);
        assert_eq!(distinct(&cards).collect::<Vec<_>>(), vec![RED, BROWN, Kind::Money(0)]);
    }
}
