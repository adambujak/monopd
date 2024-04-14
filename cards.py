from enum import Enum
import random

class CardClassType(Enum):
    ACTION = 1
    PROPERTY = 2
    WILDPROPERTY = 3
    CASH = 4

class PropertyCardColor(Enum):
    NONE   = -1
    ANY    = 0
    BROWN  = 1
    TEAL   = 2
    BLACK  = 3
    WATER  = 4
    GREEN  = 5
    BLUE   = 6
    RED    = 7
    YELLOW = 8
    PINK   = 9
    ORANGE = 10

class ActionCardTypes(Enum):
    JSN      = 1
    DEALBRK  = 2
    BIRTHDAY = 3
    DEBT     = 4
    DOUBLE   = 5
    FORCED   = 6
    SLY      = 7
    HOUSE    = 8
    HOTEL    = 9
    PASSGO   = 10
    RENT     = 11
    WILDRENT = 12

class WildPropertyCardColor(Enum):
    BLUE_GREEN  = (PropertyCardColor.BLUE, PropertyCardColor.GREEN)
    TEAL_BROWN  = (PropertyCardColor.TEAL, PropertyCardColor.BROWN)
    ORANGE_PINK = (PropertyCardColor.ORANGE, PropertyCardColor.PINK)
    GREEN_BLACK = (PropertyCardColor.GREEN, PropertyCardColor.BLACK)
    TEAL_BLACK  = (PropertyCardColor.TEAL, PropertyCardColor.BLACK)
    WATER_BLACK = (PropertyCardColor.WATER, PropertyCardColor.BLACK)
    YELLOW_RED  = (PropertyCardColor.YELLOW, PropertyCardColor.RED)
    WILD        = ()

property_charge_values = {
    PropertyCardColor.BROWN:  [1, 2],
    PropertyCardColor.WATER:  [1, 2],
    PropertyCardColor.BLUE:   [3, 8],
    PropertyCardColor.TEAL:   [1, 2, 3],
    PropertyCardColor.GREEN:  [2, 4, 7],
    PropertyCardColor.RED:    [2, 3, 6],
    PropertyCardColor.YELLOW: [2, 4, 6],
    PropertyCardColor.PINK:   [1, 2, 4],
    PropertyCardColor.ORANGE: [1, 3, 5],
    PropertyCardColor.BLACK:  [1, 2, 3, 4]
    }

property_card_values = {
    PropertyCardColor.BROWN:  1,
    PropertyCardColor.WATER:  2,
    PropertyCardColor.BLUE:   4,
    PropertyCardColor.TEAL:   1,
    PropertyCardColor.GREEN:  4,
    PropertyCardColor.RED:    3,
    PropertyCardColor.YELLOW: 3,
    PropertyCardColor.PINK:   2,
    PropertyCardColor.ORANGE: 2,
    PropertyCardColor.BLACK:  2
    }

wild_property_card_values = {
    WildPropertyCardColor.BLUE_GREEN:  4,
    WildPropertyCardColor.TEAL_BROWN:  1,
    WildPropertyCardColor.ORANGE_PINK: 2,
    WildPropertyCardColor.GREEN_BLACK: 4,
    WildPropertyCardColor.TEAL_BLACK:  4,
    WildPropertyCardColor.WATER_BLACK: 4,
    WildPropertyCardColor.YELLOW_RED:  3
    }

action_card_values = {
    ActionCardTypes.JSN:      4,
    ActionCardTypes.DEALBRK:  5,
    ActionCardTypes.BIRTHDAY: 2,
    ActionCardTypes.DEBT:     3,
    ActionCardTypes.DOUBLE:   1,
    ActionCardTypes.FORCED:   3,
    ActionCardTypes.SLY:      3,
    ActionCardTypes.HOUSE:    3,
    ActionCardTypes.HOTEL:    4,
    ActionCardTypes.PASSGO:   1,
    ActionCardTypes.RENT:     1,
    ActionCardTypes.WILDRENT: 3,
    }

unique_iter = 0

class Card:
    def __init__(self, value, card_type):
        global unique_iter
        self.value = value
        self.card_type = card_type
        self.unique_id = unique_iter
        unique_iter += 1

    def get_value(self):
        return self.value

    def is_action_card(self):
        return self.card_type == CardClassType.ACTION

    def is_property(self):
        return self.card_type == CardClassType.PROPERTY or self.card_type == CardClassType.WILDPROPERTY

    def is_cash(self):
        return self.card_type == CardClassType.CASH

    def get_unique_id(self):
        return self.unique_id

class CashCard(Card):
    def __init__(self, value):
        super().__init__(value, CardClassType.CASH)

    def __str__(self):
        return "CASH: ${}M".format(self.value)

class PropertyCard(Card):
    def __init__(self, color: PropertyCardColor):
        self.color = color
        value = property_card_values[color]
        super().__init__(value, CardClassType.PROPERTY)

    def get_color(self):
        return self.color

    def __str__(self):
        return "PROPERTY: {}".format(self.color.name)

class WildPropertyCard(Card):
    def __init__(self, colors: WildPropertyCardColor):

        self.assigned_color = None
        self.colors = colors

        if colors == WildPropertyCardColor.WILD:
            self.wild = True
            super().__init__(0, CardClassType.PROPERTY)
        else:
            self.wild = False
            value = wild_property_card_values[colors]
            super().__init__(value, CardClassType.PROPERTY)

    def get_color(self):
        return self.assigned_color

    def assign(self, color):
        if color in self.colors or self.wild:
            self.assigned_color = color

    def __str__(self):
        if self.wild:
            return "PROPERTY: Magic Rainbow Wild Card"
        else:
            if self.assigned_color:
                return "PROPERTY: Wild Card {} - assigned as {}".format(self.colors.name, self.assigned_color.name)
            else:
                return "PROPERTY: Wild Card {} - unassigned".format(self.colors.name)


class ActionCard(Card):
    def __init__(self, action: ActionCardTypes):
        self.action = action
        value = action_card_values[self.action]
        super().__init__(value, CardClassType.ACTION)

    def __str__(self):
        return "ACTION: {}".format(self.action.name)


class RentCard(ActionCard):
    def __init__(self, colors: list[PropertyCardColor]):
        self.wild = colors is None
        if self.wild:
            self.action = ActionCardTypes.WILDRENT
        else:
            self.colors = colors
            self.action = ActionCardTypes.RENT
        super().__init__(self.action)

    def get_colors(self):
        return self.colors

    def __str__(self):
        if self.wild:
            return "RENT: Wild Rent"
        else:
            return "RENT: {} or {}".format(self.colors[0].name, self.colors[1].name)

class Set:
    def __init__(self, color:PropertyCardColor):
        self.color = color
        self.charge_value = property_charge_values[color][0]
        self.count = 1
        self.full = False

    def add(self):
        if not self.full:
            self.count += 1
            self.charge_value = property_charge_values[self.color][self.count-1]
            self.full = (self.count == len(property_charge_values[self.color]))
        else:
            raise AssertionError('Set is full. Cannot add card to set')

    def get_charge_value(self):
        return self.charge_value

    def get_color(self):
        return self.color

    def is_full(self):
        return self.full

    def __str__(self):
        if self.is_full():
            return "FULL SET! - {}".format(self.color.name)
        else:
            return "SET - {} - {} properties".format(self.color.name, self.count)

class Deck:
    def __init__(self):
        self.cards = []

        # add property cards
        for c in property_charge_values:
            self.add_property_card(c, len(property_charge_values[c]))

        # add wild property cards
        self.add_wild_property_card(WildPropertyCardColor.BLUE_GREEN , 1)
        self.add_wild_property_card(WildPropertyCardColor.TEAL_BROWN , 1)
        self.add_wild_property_card(WildPropertyCardColor.ORANGE_PINK, 2)
        self.add_wild_property_card(WildPropertyCardColor.GREEN_BLACK, 1)
        self.add_wild_property_card(WildPropertyCardColor.TEAL_BLACK, 1)
        self.add_wild_property_card(WildPropertyCardColor.WATER_BLACK, 1)
        self.add_wild_property_card(WildPropertyCardColor.YELLOW_RED, 2)
        self.add_wild_property_card(WildPropertyCardColor.WILD, 2)

        # add action cards
        self.add_action_card(ActionCardTypes.DEALBRK, 2)
        self.add_action_card(ActionCardTypes.DOUBLE, 2)
        self.add_action_card(ActionCardTypes.JSN, 3)
        self.add_action_card(ActionCardTypes.BIRTHDAY, 3)
        self.add_action_card(ActionCardTypes.DEBT, 3)
        self.add_action_card(ActionCardTypes.SLY, 3)
        self.add_action_card(ActionCardTypes.HOUSE, 3)
        self.add_action_card(ActionCardTypes.HOTEL, 3)
        self.add_action_card(ActionCardTypes.FORCED, 4)
        self.add_action_card(ActionCardTypes.PASSGO, 10)

        # add rent cards
        self.add_rent_card(None, 3)
        self.add_rent_card([PropertyCardColor.GREEN, PropertyCardColor.BLUE], 2)
        self.add_rent_card([PropertyCardColor.BROWN, PropertyCardColor.TEAL], 2)
        self.add_rent_card([PropertyCardColor.PINK, PropertyCardColor.ORANGE], 2)
        self.add_rent_card([PropertyCardColor.BLACK, PropertyCardColor.WATER], 2)
        self.add_rent_card([PropertyCardColor.RED, PropertyCardColor.YELLOW], 2)

        # add cash cards
        self.add_cash_card(1, 6)
        self.add_cash_card(2, 5)
        self.add_cash_card(3, 3)
        self.add_cash_card(4, 3)
        self.add_cash_card(5, 2)
        self.add_cash_card(10, 1)

        # shuffle deck
        random.shuffle(self.cards)

    def add_card(self, card):
        self.cards.append(card)

    def add_property_card(self, color: PropertyCardColor, count):
        for _ in range(count):
            self.add_card(PropertyCard(color))

    def add_wild_property_card(self, colors: WildPropertyCardColor, count):
        for _ in range(count):
            self.add_card(WildPropertyCard(colors))

    def add_cash_card(self, value, count):
        for _ in range(count):
            self.add_card(CashCard(value))

    def add_action_card(self, action: ActionCardTypes, count):
        for _ in range(count):
            self.add_card(ActionCard(action))

    def add_rent_card(self, colors, count):
        for _ in range(count):
            self.add_card(RentCard(colors))

    def is_empty(self):
        return len(self.cards) == 0

    def draw_card(self):
        if self.is_empty():
            return None
        return self.cards.pop(random.randrange(len(self.cards)))
