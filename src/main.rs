// Code to solve Rock Paper Scissors using Counterfactual Regret Minimization
// Following "An Introduction to Counterfactual Regret Minimization" by Neller and Lanctot (2013)

use rand::Rng;
use std::collections::HashMap;

fn main() {
    let num_iters = 1_000;
    let mut rng = rand::thread_rng();
    let mut deck: [Card; NUM_CARDS] = [Card::Ace, Card::King, Card::Queen];

    let mut util = 0.0;
    let mut node_map: HashMap<InfoSet, NodeInfo> = HashMap::new();

    for _ in 0..num_iters {
        shuffle_deck(&mut deck, &mut rng);
        util += cfr(&deck, &mut node_map, new_history(), 1.0, 1.0);
    }

    // for debugging purposes
    let base_set = InfoSet {
        player: Player::Player0,
        card: Card::Ace,
        history: new_history(),
    };
    node_map.insert(
        base_set,
        NodeInfo {
            regret_sum: HashMap::new(),
            strategy: HashMap::new(),
            strategy_sum: HashMap::new(),
        },
    );

    println!("Average game value is {}", util / (num_iters as f64));
    for info_set in node_map.keys() {
        let avg_strategy = node_map
            .get(&info_set)
            .expect("We should be indexing by keys that actually exist")
            .get_average_strategy();
        println!(
            "At info_set {:?}, avg strategy is {:?}",
            info_set, avg_strategy
        );
    }
}

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
enum Card {
    Queen,
    King,
    Ace,
}
const NUM_CARDS: usize = 3;

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
        assert!(!(Card::Queen > Card::King));
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Move {
    Pass,
    Bet,
}
const MOVE_LIST: [Move; 2] = [Move::Pass, Move::Bet];

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum Player {
    Player0,
    Player1,
}
const PLAYER_LIST: [Player; 2] = [Player::Player0, Player::Player1];

fn other_player(p: Player) -> Player {
    match p {
        Player::Player0 => Player::Player1,
        Player::Player1 => Player::Player0,
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct History {
    player_to_move: Player,
    moves: Vec<Move>,
}

fn new_history() -> History {
    History {
        player_to_move: Player::Player0,
        moves: Vec::new(),
    }
}

impl History {
    fn append(&mut self, p: Player, m: Move) {
        if p != self.player_to_move {
            panic!("Attempted to add to history, but it was the wrong player's turn");
        } else {
            self.player_to_move = other_player(p);
            self.moves.push(m);
        }
    }
}

#[cfg(test)]
mod history_tests {
    use crate::{new_history, History, Move, Player};

    #[test]
    fn player_append_valid() {
        let mut my_hist = new_history();
        my_hist.append(Player::Player0, Move::Pass);
        assert_eq!(
            my_hist,
            History {
                player_to_move: Player::Player1,
                moves: vec![Move::Pass]
            }
        );
    }

    #[test]
    #[should_panic]
    fn player_append_invalid() {
        let mut my_hist = new_history();
        my_hist.append(Player::Player1, Move::Bet);
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct InfoSet {
    player: Player,
    card: Card,
    history: History,
}
// TODO: make sure we can't make invalid info sets? Hopefully?

struct NodeInfo {
    regret_sum: HashMap<Move, f64>,
    strategy: HashMap<Move, f64>,
    strategy_sum: HashMap<Move, f64>,
}

impl NodeInfo {
    fn get_strategy(&mut self, realization_weight: f64) -> &HashMap<Move, f64> {
        let mut normalizing_sum = 0.0;
        for m in MOVE_LIST {
            let r = self.regret_sum.get(&m).unwrap_or_else(|| &0.0);
            let r_pos = if *r > 0.0 { *r } else { 0.0 };
            self.strategy.insert(m, r_pos);
            normalizing_sum += r_pos;
        }
        for m in MOVE_LIST {
            let strat_m = if normalizing_sum > 0.0 {
                let s = self
                    .strategy
                    .get(&m)
                    .expect("We should have supplied this value earlier in this function.");
                s / normalizing_sum
            } else {
                1.0 / (MOVE_LIST.len() as f64)
            };
            self.strategy.insert(m, strat_m);
            let s_sum = self.strategy_sum.get(&m).unwrap_or_else(|| &0.0);
            self.strategy_sum
                .insert(m, s_sum + realization_weight * strat_m);
        }

        &self.strategy
    }

    fn get_average_strategy(&self) -> HashMap<Move, f64> {
        let mut avg_strategy: HashMap<Move, f64> = HashMap::new();
        let mut normalizing_sum = 0.0;
        for m in MOVE_LIST {
            normalizing_sum += self.strategy_sum.get(&m).unwrap_or_else(|| &0.0);
        }
        for m in MOVE_LIST {
            avg_strategy.insert(
                m,
                if normalizing_sum > 0.0 {
                    self.strategy_sum.get(&m).unwrap_or_else(|| &0.0) / normalizing_sum
                } else {
                    1.0 / (MOVE_LIST.len() as f64)
                },
            );
        }
        avg_strategy
    }
}

fn shuffle_deck(deck: &mut [Card; NUM_CARDS], rng: &mut rand::rngs::ThreadRng) {
    for i in (1..NUM_CARDS).rev() {
        let j = rng.gen_range(0..(i + 1));
        let tmp = deck[j];
        deck[j] = deck[i];
        deck[i] = tmp;
    }
}

fn cfr(
    deck: &[Card; NUM_CARDS],
    node_map: &mut HashMap<InfoSet, NodeInfo>,
    hist: History,
    prob_0: f64,
    prob_1: f64,
) -> f64 {
    // TODO: actually complete
    return 0.0;
}

// how do I generate info sets?
// probably want to have a permissive type that I gradually populate?
// make type for times? actually that's for the algorithm, not for the game.
// history = sequence of actions
// need to know whether a history is terminal
// strategy for an info set is probs over each move.

// probably makes sense to implement normalization in sampling code?
// oh except it messes up the averaging.

// counterfactual value or player i given strategies sigma and history h v_i(sigma, h) is:
// prob(reach h given -i plays sigma_{-i}, i plays exactly what they played deterministically)
// * sum_{terminal states extending h} prob(reach z from h) * u_i(z);
// (in kuhn poker, we're summing this, not sampling)
// counterfactual rergret of a at h for i, r_i(h, a) = v_i(sigma_{I -> a}, h) - v_i(sigma, h)
// counterfactual regret of a at I for i, r_i(I, a) is sum_{h in I} r_i(h, a)

// sample from chance nodes
// need to know which player is next at a given history

// way you get strategy is cumulative regret matching

// so: for each info set and player, want a regret table, strategy table.
// probably implement with a hash map with default value of [0, 0]
