from cards import *

players = ["adam", "devin"]

class Pile:
    def __init__(self):
        self.cards = []

    def add_card(self, card):
        self.cards.append(card)

    def remove_card(self, index):
        return self.cards.pop(index)

    def get_cards(self):
        return self.cards

    def peek_card(self, index):
        return self.cards[index]

    def __str__(self):
        outstr = "pile: \n"
        for card in self.cards:
            outstr += "\t{}\n".format(str(card))
        return outstr

class Hand(Pile):
    def get_number_of_cards(self):
        return len(self.cards)

    def __str__(self):
        outstr = "HAND: \n"
        for i, card in enumerate(self.cards):
            outstr += "\t{}. {}\n".format(i, str(card))
        return outstr

class CashPile(Pile):
    def __str__(self):
        outstr = "CASH PILE: \n"
        for card in self.cards:
            outstr += "\t{}\n".format(str(card))
        return outstr

class DiscardPile(Pile):
    def __str__(self):
        outstr = "DISCARD PILE: \n"
        for card in self.cards:
            outstr += "\t{}\n".format(str(card))
        return outstr

class PropertyPile(Pile):
    def __init__(self):
        self.sets = []
        super().__init__()

    def add_card(self, card: PropertyCard):
        self.cards.append(card)
        for s in self.sets:
            if s.get_color() == card.get_color() and not s.is_full():
                s.add()
                return
        new_set = Set(card.get_color())
        self.sets.append(new_set)

class Player:
    def __init__(self, name):
        self.name = name
        self.hand = Hand()
        self.property_pile = PropertyPile()
        self.cash_pile = CashPile()

    def deal_card(self, card):
        self.hand.add_card(card)

    def give_card(self, card):
        if card.card_type == CardClassType.PROPERTY:
            self.property_pile.add_card(card)
        elif card.card_type == CardClassType.ACTION:
            self.cash_pile.add_card(card)
        elif card.card_type == CardClassType.CASH:
            self.cash_pile.add_card(card)

    def give_cards(self, cards):
        for card in cards:
            self.give_card(card)

    def move_card_to_properties(self, index):
        card = self.hand.remove_card(index)
        if card.card_type != CardClassType.PROPERTY:
            raise AssertionError("Cannot put non property card into property pile")
        else:
            self.property_pile.add_card(card)

    def move_card_to_cash(self, index):
        card = self.hand.remove_card(index)
        self.cash_pile.add_card(card)

    def get_card(self, index):
        return self.hand.remove_card(index)

    def charge(self, amount):
        cards = []
        total = 0
        # TODO: allow user to select cards
        for i in range(len(self.cash_pile.get_cards())):
            if total >= amount:
                return cards
            card = self.cash_pile.remove_card(i)
            total += card.value
            cards.append(card)

        for i in range(len(self.property_pile.get_cards())):
            if total >= amount:
                return cards
            card = self.property_pile.remove_card(i)
            total += card.value
            cards.append(card)

        return cards

    def has_won(self):
        full_sets = 0
        for s in self.property_pile.sets:
            if s.is_full():
                full_sets += 1
        if full_sets >= 3:
            return True
        else:
            return False

    def __str__(self):
        return "{}:\n{}".format(self.name, str(self.hand) + str(self.cash_pile) + str(self.property_pile))

class Game:
    def __init__(self, players) -> None:
        self.deck = Deck()
        self.discard_pile = DiscardPile()
        self.players = []
        for player in players:
            self.players.append(Player(player))

        self.initial_deal()
        self.active_player = 0
        self.game_over = False

    def initial_deal(self):
        for _ in range(5):
            for player in self.players:
                player.deal_card(self.deck.draw_card())

    def draw_2_cards(self, player):
        player.deal_card(self.deck.draw_card())
        player.deal_card(self.deck.draw_card())

    def start_turn(self):
        if self.game_over:
            raise AssertionError("Game over - cannot start new turn")
        self.draw_2_cards(self.players[self.active_player])
        #print(self.players[self.active_player])

    def play_card(self, index, cash:bool):
        player = self.players[self.active_player]
        card = player.hand.peek_card(index)
        if (cash and card.card_type == CardClassType.ACTION) or card.card_type == CardClassType.CASH:
            player.move_card_to_cash(index)
        elif card.card_type == CardClassType.PROPERTY:
            player.move_card_to_properties(index)
        elif card.card_type == CardClassType.ACTION:
            card = player.get_card(index)
            self.card_played_callback(card)
            self.discard_pile.add_card(card)

    def card_played_callback(self, card):
        if card.card_type == CardClassType.ACTION:
            if card.action == ActionCardTypes.BIRTHDAY:
                active_player = self.players[self.active_player]
                for p in self.players:
                    if active_player == p:
                        continue
                    active_player.give_cards(p.charge(2))
            if card.action == ActionCardTypes.PASSGO:
                active_player = self.players[self.active_player]
                self.draw_2_cards(active_player)

    def end_turn(self):
        player = self.players[self.active_player]
        if player.has_won():
            print(player.name, "has won!!")
            self.game_over = True
        else:
            self.active_player = (self.active_player + 1) % len(self.players)

def handle_turn(game):
    turn_count = 0
    aborted = False
    while not aborted and turn_count < 3:
        print(game.players[game.active_player])
        print('Play card: p {index}, End turn: e')
        x = input()
        if x == 'e':
            aborted = True
            continue
        a = x.split(' ')
        if a[0] == 'p':
            game.play_card(int(a[1]), False)
            turn_count += 1
        else:
            print("invalid input")
    game.end_turn()


game = Game(players)

game.start_turn()
handle_turn(game)

game.start_turn()
handle_turn(game)

game.start_turn()
handle_turn(game)

game.start_turn()
handle_turn(game)
