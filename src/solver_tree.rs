// Types of histories and nodes etc used in the counterfactual regret minimization algorithm.
// Includes relevant methods.

use std::collections::HashMap;

use crate::game::{get_player_card, other_player, winning_player, Card, Move, Player, NUM_CARDS};

pub type Floating = f64;

#[derive(Debug)]
enum HistState {
    InProgress,
    DoubleCheck,
    Fold,
    Showdown,
}

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
            let terminal_check = moves[length - 1] == Move::Check;
            let terminal_fold = moves[length - 1] == Move::Fold;
            let terminal_call = moves[length - 1] == Move::Call;
            if terminal_check {
                if length == 2 && moves[0] == Move::Check {
                    return HistState::DoubleCheck;
                } else {
                    panic!("Player should not be allowed to check after move 2, or when previous move is not a check.")
                }
            } else if terminal_fold {
                return HistState::Fold;
            } else if terminal_call {
                return HistState::Showdown;
            }
        }
        HistState::InProgress
    }

    fn next_moves(&self) -> Vec<Move> {
        // return a vector of legal next moves
        let length = self.len();
        if length > 0 {
            match self.moves[length - 1] {
                Move::Check => vec![Move::Check, Move::Bet],
                Move::Bet => vec![Move::Call, Move::Raise, Move::Fold],
                Move::Call => Vec::new(),
                Move::Raise => vec![Move::Call, Move::Fold],
                Move::Fold => Vec::new(),
            }
        } else {
            vec![Move::Check, Move::Bet]
        }
    }
}

#[derive(Debug)]
pub struct ChancyHistory {
    player_to_move: Player,
    moves_and_counterfactual_reach_probs: Vec<((Floating, Floating), Move)>,
}

#[cfg(test)]
mod chancy_hist_tests {
    use crate::game::{Card, Move, Player};
    use crate::solver_tree::{ChancyHistory, History};

    #[test]
    fn player_append_valid() {
        let chancy_hist_0 = ChancyHistory::new();
        let chancy_hist_1 = chancy_hist_0.extend(Move::Check, 0.8).unwrap();
        let chancy_hist_2 = chancy_hist_1.extend(Move::Bet, 0.7).unwrap();
        assert_eq!(
            chancy_hist_2.determinize(),
            History {
                player_to_move: Player::Player0,
                moves: vec![Move::Check, Move::Bet]
            }
        );
    }

    #[test]
    #[should_panic]
    fn extend_invalid() {
        let my_chancy_hist = ChancyHistory::new();
        my_chancy_hist.extend(Move::Call, 0.8).unwrap();
    }

    #[test]
    fn right_reach_probs() {
        let chancy_hist_0 = ChancyHistory::new();
        let chancy_hist_1 = chancy_hist_0.extend(Move::Check, 0.8).unwrap();
        let chancy_hist_2 = chancy_hist_1.extend(Move::Bet, 0.7).unwrap();
        assert_eq!(chancy_hist_0.get_reach_prob(), 1.0);
        assert_eq!(chancy_hist_2.get_reach_prob(), 0.8 * 0.7);
        assert_eq!(chancy_hist_2.get_counterfactual_reach_prob(), 0.7);
    }

    #[test]
    fn right_terminal_utilities() {
        let chancy_hist_0 = ChancyHistory::new();
        let chancy_hist_1 = chancy_hist_0.extend(Move::Check, 0.5).unwrap();
        let chancy_hist_2 = chancy_hist_1.extend(Move::Bet, 0.5).unwrap();
        let chancy_hist_3 = chancy_hist_2.extend(Move::Call, 0.5).unwrap();
        let deck = [Card::Ace, Card::King, Card::Queen, Card::Jack, Card::Ten];
        assert_eq!(chancy_hist_2.util_if_terminal(&deck), None);
        assert_eq!(chancy_hist_3.util_if_terminal(&deck), Some(-2.0));
    }
}

impl ChancyHistory {
    pub fn new() -> Self {
        Self {
            player_to_move: Player::Player0,
            moves_and_counterfactual_reach_probs: Vec::new(),
        }
    }

    fn determinize(&self) -> History {
        // Return a history with the probabilities stripped out
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
        // get the utility of terminal histories, return None if not terminal.
        let current_player_winning = winning_player(deck) == self.player_to_move;
        let has_raise = self.determinize().moves.contains(&Move::Raise);
        match self.determinize().termination_type() {
            HistState::InProgress => None,
            HistState::DoubleCheck => Some(if current_player_winning { 1.0 } else { -1.0 }),
            HistState::Fold => Some(if has_raise { 2.0 } else { 1.0 }),
            HistState::Showdown => Some(
                if has_raise { 3.0 } else { 2.0 } * if current_player_winning { 1.0 } else { -1.0 },
            ),
        }
    }

    pub fn extend(&self, m: Move, prob: Floating) -> Option<Self> {
        // add a move to the history
        // return None if it was an illegal move
        let new_player = other_player(self.player_to_move);
        let legal_moves = self.determinize().next_moves();
        if !legal_moves.contains(&m) {
            return None;
        }
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
        Some(Self {
            player_to_move: new_player,
            moves_and_counterfactual_reach_probs: new_moves_probs,
        })
    }

    pub fn get_reach_prob(&self) -> Floating {
        // returns the probability of reaching this history
        let length = self.len();
        if length == 0 {
            1.0
        } else {
            let &(prob_0, prob_1) = &self.moves_and_counterfactual_reach_probs[length - 1].0;
            prob_0 * prob_1
        }
    }

    pub fn get_counterfactual_reach_prob(&self) -> Floating {
        // returns the probability of reaching this history if the current player deterministically
        // played the moves they in fact played
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
        !matches!(self.history.termination_type(), HistState::InProgress)
    }

    pub fn get_next_moves(&self) -> Vec<Move> {
        self.history.next_moves()
    }
}

pub struct NodeInfo {
    regret_sum: HashMap<Move, Floating>,
    strategy: HashMap<Move, Floating>,
    strategy_sum: HashMap<Move, Floating>,
}

impl NodeInfo {
    pub fn new(legal_moves: &Vec<Move>) -> Self {
        Self {
            regret_sum: new_move_to_float_map_zeros(legal_moves),
            strategy: new_move_to_float_map_probs(legal_moves),
            strategy_sum: new_move_to_float_map_zeros(legal_moves),
        }
    }

    pub fn get_strategy(&self, m: Move) -> Floating {
        *self
            .strategy
            .get(&m)
            .expect("All nodes that exist should have strategies, and we should only call get_strategy on legal moves")
    }

    pub fn update_regret(&mut self, m: Move, r: Floating) {
        *self.regret_sum.get_mut(&m).expect(
            "We should only call update_regret on nodes that have regret sums, and on legal moves",
        ) += r;
    }

    pub fn update_strategy(&mut self, legal_moves: &Vec<Move>, realization_weight: Floating) {
        // compute strategies by regret matching
        let mut normalizing_sum = 0.0;
        for m in legal_moves {
            let r = self.regret_sum.get(m).unwrap_or(&0.0);
            let r_pos = if *r > 0.0 { *r } else { 0.0 };
            self.strategy.insert(*m, r_pos);
            normalizing_sum += r_pos;
        }
        for m in legal_moves {
            let strat_m = if normalizing_sum > 0.0 {
                self.strategy.get(m).unwrap() / normalizing_sum
            } else {
                1.0 / (legal_moves.len() as Floating)
            };
            self.strategy.insert(*m, strat_m);
            let sum_update = realization_weight * strat_m;
            self.strategy_sum
                .entry(*m)
                .and_modify(|s| *s += sum_update)
                .or_insert(sum_update);
        }
    }

    pub fn get_average_strategy(&self, legal_moves: &Vec<Move>) -> HashMap<Move, Floating> {
        let mut avg_strategy: HashMap<Move, Floating> = HashMap::new();
        let mut normalizing_sum = 0.0;
        for m in legal_moves {
            normalizing_sum += self.strategy_sum.get(m).unwrap_or(&0.0);
        }
        for m in legal_moves {
            avg_strategy.insert(
                *m,
                if normalizing_sum > 0.0 {
                    self.strategy_sum.get(m).unwrap_or(&0.0) / normalizing_sum
                } else {
                    1.0 / (legal_moves.len() as Floating)
                },
            );
        }
        avg_strategy
    }
}

fn new_move_to_float_map_zeros(legal_moves: &Vec<Move>) -> HashMap<Move, Floating> {
    let mut new_map = HashMap::new();
    for m in legal_moves {
        new_map.insert(*m, 0.0);
    }
    new_map
}

fn new_move_to_float_map_probs(legal_moves: &Vec<Move>) -> HashMap<Move, Floating> {
    let mut new_map = HashMap::new();
    let num_moves = legal_moves.len();
    for m in legal_moves {
        new_map.insert(*m, 1.0 / (num_moves as Floating));
    }
    new_map
}

pub struct NodeUtils {
    pub value: Floating,
    pub move_utils: HashMap<Move, Floating>,
}

impl NodeUtils {
    pub fn new(legal_moves: &Vec<Move>) -> Self {
        Self {
            value: 0.0,
            move_utils: new_move_to_float_map_zeros(legal_moves),
        }
    }
}
