// Types of histories and nodes etc used in the counterfactual regret minimization algorithm.
// Includes relevant methods.

use std::collections::HashMap;

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
    fn len(&self) -> usize {
        self.moves.len()
    }

    fn termination_type(&self) -> HistState {
        let length = self.len();
        let moves = &self.moves;
        if length > 1 {
            let terminal_pass = moves[length - 1] == Move::Pass;
            let double_bet = moves[length - 1] == Move::Bet && moves[length - 2] == Move::Bet;
            if terminal_pass {
                if length == 2 && moves[0] == Move::Pass {
                    return HistState::DoublePass;
                } else {
                    return HistState::BetFold;
                }
            } else if double_bet {
                return HistState::Showdown;
            }
        }
        HistState::InProgress
    }
}

#[derive(Debug)]
pub struct ChancyHistory {
    player_to_move: Player,
    moves_and_counterfactual_reach_probs: Vec<((Floating, Floating), Move)>,
}

#[derive(Debug)]
enum HistState {
    InProgress,
    DoublePass,
    BetFold,
    Showdown,
}

impl ChancyHistory {
    pub fn new() -> Self {
        Self {
            player_to_move: Player::Player0,
            moves_and_counterfactual_reach_probs: Vec::new(),
            // note that on the first move we don't have explicit counterfactual probabilities
            // of 1.0, sorry.
        }
    }

    fn determinize(&self) -> History {
        History {
            player_to_move: self.player_to_move,
            moves: self
                .moves_and_counterfactual_reach_probs
                .iter()
                .map(|&(_, m)| m)
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

    pub fn is_terminal(&self) -> bool {
        !matches!(self.determinize().termination_type(), HistState::InProgress)
    }

    pub fn util_if_terminal(&self, deck: &[Card; NUM_CARDS]) -> Option<Floating> {
        let current_player_winning = winning_player(deck) == self.player_to_move;
        match self.determinize().termination_type() {
            HistState::InProgress => None,
            HistState::DoublePass => Some(if current_player_winning { 1.0 } else { -1.0 }),
            HistState::BetFold => Some(1.0),
            HistState::Showdown => Some(if current_player_winning { 2.0 } else { -2.0 }),
        }
    }

    pub fn extend(&self, m: Move, prob: Floating) -> Self {
        let new_player = other_player(self.player_to_move);
        let length = self.len();
        let counterfac_probs = if length == 0 {
            assert!(
                self.player_to_move == Player::Player0,
                "Zero-length history with player 1 to move somehow got initialized"
            );
            (prob, 1.0)
        } else {
            let &(prob_0, prob_1) = &self.moves_and_counterfactual_reach_probs[length - 1].0;
            match self.player_to_move {
                Player::Player0 => (prob_0 * prob, prob_1),
                Player::Player1 => (prob_0, prob_1 * prob),
            }
        };
        let mut new_moves_probs = self.moves_and_counterfactual_reach_probs.clone();
        new_moves_probs.push((counterfac_probs, m));
        Self {
            player_to_move: new_player,
            moves_and_counterfactual_reach_probs: new_moves_probs,
        }
    }

    pub fn get_reach_prob(&self) -> Floating {
        let length = self.len();
        if length == 0 {
            1.0
        } else {
            let &(prob_0, prob_1) = &self.moves_and_counterfactual_reach_probs[length - 1].0;
            prob_0 * prob_1
        }
    }

    pub fn get_counterfactual_reach_prob(&self) -> Floating {
        let length = self.len();
        if length == 0 {
            1.0
        } else {
            let &(prob_0, prob_1) = &self.moves_and_counterfactual_reach_probs[length - 1].0;
            match self.player_to_move {
                Player::Player0 => prob_1,
                Player::Player1 => prob_0,
            }
        }
    }
}

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub struct InfoSet {
    card: Card,
    history: History,
}

impl InfoSet {
    pub fn is_terminal(&self) -> bool {
        // make termination type a function of history
        // propagate those changes
        !matches!(self.history.termination_type(), HistState::InProgress)
    }
}

pub struct NodeInfo {
    regret_sum: HashMap<Move, Floating>,
    strategy: HashMap<Move, Floating>,
    strategy_sum: HashMap<Move, Floating>,
}

impl NodeInfo {
    pub fn new() -> Self {
        Self {
            regret_sum: new_move_to_float_map_zeros(),
            strategy: new_move_to_float_map_probs(),
            strategy_sum: new_move_to_float_map_zeros(),
        }
    }

    pub fn get_strategy(&self, m: Move) -> Floating {
        *self
            .strategy
            .get(&m)
            .expect("All nodes that exist should have strategies")
    }

    pub fn update_regret(&mut self, m: Move, r: Floating) {
        *self
            .regret_sum
            .get_mut(&m)
            .expect("We should only call update_regret on nodes that have regret sums") += r;
    }

    pub fn update_strategy(&mut self, realization_weight: Floating) {
        // compute strategies by regret matching
        let mut normalizing_sum = 0.0;
        for m in MOVE_LIST {
            let r = self.regret_sum.get(&m).unwrap_or(&0.0);
            let r_pos = if *r > 0.0 { *r } else { 0.0 };
            self.strategy.insert(m, r_pos);
            normalizing_sum += r_pos;
        }
        for m in MOVE_LIST {
            let strat_m = if normalizing_sum > 0.0 {
                self.strategy.get(&m).unwrap() / normalizing_sum
            } else {
                1.0 / (MOVE_LIST.len() as Floating)
            };
            self.strategy.insert(m, strat_m);
            let sum_update = realization_weight * strat_m;
            self.strategy_sum
                .entry(m)
                .and_modify(|s| *s += sum_update)
                .or_insert(sum_update);
        }
    }

    pub fn get_average_strategy(&self) -> HashMap<Move, Floating> {
        let mut avg_strategy: HashMap<Move, Floating> = HashMap::new();
        let mut normalizing_sum = 0.0;
        for m in MOVE_LIST {
            normalizing_sum += self.strategy_sum.get(&m).unwrap_or(&0.0);
        }
        for m in MOVE_LIST {
            avg_strategy.insert(
                m,
                if normalizing_sum > 0.0 {
                    self.strategy_sum.get(&m).unwrap_or(&0.0) / normalizing_sum
                } else {
                    1.0 / (MOVE_LIST.len() as Floating)
                },
            );
        }
        avg_strategy
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
