// Types of histories and nodes etc used in the counterfactual regret minimization algorithm.
// Includes relevant methods.

use std::collections::{HashMap, VecDeque};

use crate::game::{
    get_player_card, other_player, winning_player, Card, Move, Player, MOVE_LIST, NUM_CARDS,
};

pub type Floating = f64;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct History {
    player_to_move: Player,
    moves: Vec<Move>,
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

    fn retract(&mut self) {
        let p = self.player_to_move;
        self.player_to_move = other_player(p);
        self.moves.pop();
    }

    fn truncate(&mut self, n: usize) {
        if n > self.moves.len() {
            panic!("Attempted to truncate a history to a limit longer than its length.")
        }
        self.moves.truncate(n);
        if n % 2 != 0 {
            self.player_to_move = other_player(self.player_to_move);
        }
    }

    fn new() -> Self {
        Self {
            player_to_move: Player::Player0,
            moves: Vec::new(),
        }
    }

    fn get_info_set(&self, deck: &[Card; NUM_CARDS]) -> InfoSet {
        let card = get_player_card(self.player_to_move, deck);
        InfoSet {
            card: card,
            history: self.clone(),
        }
    }
}

#[cfg(test)]
mod history_tests {
    use crate::{History, Move, Player};

    #[test]
    fn player_append_valid() {
        let mut my_hist = History::new();
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
        let mut my_hist = History::new();
        my_hist.append(Player::Player1, Move::Bet);
    }
}

#[derive(Debug)]
pub struct ChancyHistory {
    player_to_move: Player,
    moves_and_counterfactual_reach_probs: Vec<((Floating, Floating), Move)>,
}

impl ChancyHistory {
    pub fn truncate(&self, n: usize, deck: &[Card; NUM_CARDS]) -> (InfoSet, Move) {
        if n >= self.len() {
            panic!("Attempted to truncate a ChancyHistory to a limit no shorter than its length");
        }
        let mut trunc_moves: Vec<Move> = self
            .moves_and_counterfactual_reach_probs
            .iter()
            .map(|&((p0, p1), m)| m)
            .collect();
        let mut rest_moves = VecDeque::from(trunc_moves.split_off(n));
        let next_move = rest_moves
            .pop_front()
            .expect("We should truncate to a proper subset of the history");
        let new_player = if n % 2 == 0 {
            self.player_to_move
        } else {
            other_player(self.player_to_move)
        };
        let card = get_player_card(new_player, deck);
        let info_set = InfoSet {
            card: card,
            history: History {
                player_to_move: new_player,
                moves: trunc_moves,
            },
        };
        (info_set, next_move)
    }

    fn determinize(&self) -> History {
        History {
            player_to_move: self.player_to_move,
            moves: self
                .moves_and_counterfactual_reach_probs
                .iter()
                .map(|&((p0, p1), m)| m)
                .collect(),
        }
    }

    pub fn to_info_set(&self, deck: &[Card; NUM_CARDS]) -> InfoSet {
        let history = self.determinize();
        let card = get_player_card(self.player_to_move, deck);
        InfoSet { card, history }
    }

    pub fn len(&self) -> usize {
        self.moves_and_counterfactual_reach_probs.len()
    }

    pub fn util_if_terminal(&self, deck: &[Card; NUM_CARDS]) -> Option<Floating> {
        let length = self.len();
        let moves_etc = &self.moves_and_counterfactual_reach_probs;
        if length > 1 {
            let terminal_pass = moves_etc[length - 1].1 == Move::Pass;
            let double_bet =
                moves_etc[length - 1].1 == Move::Bet && moves_etc[length - 2].1 == Move::Bet;
            let current_player_winning = winning_player(deck) == self.player_to_move;
            if terminal_pass {
                if length == 2 && moves_etc[0].1 == Move::Pass {
                    return Some(if current_player_winning { 1.0 } else { -1.0 });
                } else {
                    return Some(1.0);
                }
            } else if double_bet {
                return Some(if current_player_winning { 2.0 } else { -2.0 });
            }
        }
        None
    }

    pub fn new() -> Self {
        Self {
            player_to_move: Player::Player0,
            moves_and_counterfactual_reach_probs: Vec::new(),
            // note that on the first move we don't have explicit counterfactual probabilities
            // of 1.0, sorry.
            // could probably implement a 'get_latest_p0_p1' method
            // actually that's going to be useful when adding children
        }
    }
}

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub struct InfoSet {
    card: Card,
    history: History,
}

pub struct NodeInfo {
    regret_sum: HashMap<Move, Floating>,
    strategy: HashMap<Move, Floating>,
    strategy_sum: HashMap<Move, Floating>,
}

impl NodeInfo {
    // TODO: move all this updating to when you accumulate the cumulative regret
    pub fn get_strategy(&self, m: Move) -> Floating {
        *self
            .strategy
            .get(&m)
            .expect("All nodes that exist should have strategies")
    }
    // pub fn get_strategy(&mut self, realization_weight: Floating) -> &HashMap<Move, Floating> {
    //     // compute strategies by regret matching
    //     let mut normalizing_sum = 0.0;
    //     for m in MOVE_LIST {
    //         let r = self.regret_sum.get(&m).unwrap_or_else(|| &0.0);
    //         let r_pos = if *r > 0.0 { *r } else { 0.0 };
    //         self.strategy.insert(m, r_pos);
    //         normalizing_sum += r_pos;
    //     }
    //     for m in MOVE_LIST {
    //         let strat_m = if normalizing_sum > 0.0 {
    //             let s = self
    //                 .strategy
    //                 .get(&m)
    //                 .expect("We should have supplied this value earlier in this function.");
    //             s / normalizing_sum
    //         } else {
    //             1.0 / (MOVE_LIST.len() as Floating)
    //         };
    //         self.strategy.insert(m, strat_m);
    //         let sum_update = realization_weight * strat_m;
    //         self.strategy_sum
    //             .entry(m)
    //             .and_modify(|s| *s += sum_update)
    //             .or_insert(sum_update);
    //     }

    //     &self.strategy
    // }

    pub fn get_average_strategy(&self) -> HashMap<Move, Floating> {
        let mut avg_strategy: HashMap<Move, Floating> = HashMap::new();
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
                    1.0 / (MOVE_LIST.len() as Floating)
                },
            );
        }
        avg_strategy
    }

    pub fn new() -> Self {
        Self {
            regret_sum: new_move_to_float_map_zeros(),
            strategy: new_move_to_float_map_probs(),
            strategy_sum: new_move_to_float_map_zeros(),
        }
    }
}

fn new_move_to_float_map_zeros() -> HashMap<Move, Floating> {
    let mut new_map = HashMap::new();
    for m in MOVE_LIST {
        new_map.insert(m, 0.0);
    }
    new_map
}

fn new_move_to_float_map_probs() -> HashMap<Move, Floating> {
    let mut new_map = HashMap::new();
    let num_moves = MOVE_LIST.len();
    for m in MOVE_LIST {
        new_map.insert(m, 1.0 / (num_moves as Floating));
    }
    new_map
}

pub struct NodeUtils {
    pub value: Floating,
    pub move_utils: HashMap<Move, Floating>,
}

impl NodeUtils {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            move_utils: new_move_to_float_map_zeros(),
        }
    }
}
