// types and methods implementing core game logic

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Card {
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}
pub const NUM_CARDS: usize = 5;

#[cfg(test)]
mod card_tests {
    use crate::Card;

    #[test]
    fn card_eq() {
        assert_eq!(Card::King, Card::King);
        assert_ne!(Card::Ace, Card::Queen);
    }

    #[test]
    fn card_ord() {
        assert!(Card::Ace > Card::Queen);
        assert!(Card::King < Card::Ace);
        assert!(Card::Queen <= Card::King);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Move {
    Check,
    Bet,
    Call,
    Raise,
    Fold,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Player {
    Player0,
    Player1,
}

pub fn other_player(p: Player) -> Player {
    match p {
        Player::Player0 => Player::Player1,
        Player::Player1 => Player::Player0,
    }
}

pub fn get_player_card(p: Player, deck: &[Card; NUM_CARDS]) -> Card {
    match p {
        Player::Player0 => deck[0],
        Player::Player1 => deck[1],
    }
}

pub fn winning_player(deck: &[Card; NUM_CARDS]) -> Player {
    let card0 = get_player_card(Player::Player0, deck);
    let card1 = get_player_card(Player::Player1, deck);
    if card0 > card1 {
        Player::Player0
    } else {
        Player::Player1
    }
}
