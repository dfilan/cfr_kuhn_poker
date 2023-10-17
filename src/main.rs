// Code to solve Rock Paper Scissors using Counterfactual Regret Minimization
// Following "An Introduction to Counterfactual Regret Minimization" by Neller and Lanctot (2013)

// TODO: make CFR iterative rather than recursive
// TODO: break out card, move, player, history etc types into their own file
// TODO document some stuff

use rand::Rng;
use std::collections::HashMap;

use crate::game::{Card, MOVE_LIST, NUM_CARDS};
use crate::solver_tree::{util_if_terminal, ChancyHistory, Floating, InfoSet, NodeInfo};

mod game;
mod solver_tree;

fn main() {
    let num_iters = 1_000;
    let mut rng = rand::thread_rng();
    let mut deck: [Card; NUM_CARDS] = [Card::Ace, Card::King, Card::Queen];

    let mut util = 0.0;
    let mut node_map: HashMap<InfoSet, NodeInfo> = HashMap::new();

    for _ in 0..num_iters {
        let mut hist = History::new();
        shuffle_deck(&mut deck, &mut rng);
        util += cfr_recursive(&deck, &mut node_map, &mut hist, 1.0, 1.0);
    }

    // for debugging purposes
    // let base_set = InfoSet {
    //     card: Card::Ace,
    //     history: History::new(),
    // };
    // node_map.insert(base_set, NodeInfo::new());

    println!("Average game value is {}", util / (num_iters as Floating));
    // for (info_set, node_info) in node_map.into_iter() {
    //     let avg_strategy = node_info.get_average_strategy();
    //     println!(
    //         "At info_set {:?}, avg strategy is {:?}",
    //         info_set, avg_strategy
    //     );
    // }
}

fn shuffle_deck(deck: &mut [Card; NUM_CARDS], rng: &mut rand::rngs::ThreadRng) {
    for i in (1..NUM_CARDS).rev() {
        let j = rng.gen_range(0..(i + 1));
        let tmp = deck[j];
        deck[j] = deck[i];
        deck[i] = tmp;
    }
}

fn cfr_recursive(
    deck: &[Card; NUM_CARDS],
    node_map: &mut HashMap<InfoSet, NodeInfo>,
    hist: &mut History,
    prob_0: f64,
    prob_1: f64,
) -> f64 {
    // TODO: re-write it in DFS style to not be recursive
    // TODO: write check that the length of hist is the same at the start and end of this function body

    // return utility of terminal nodes
    match util_if_terminal(hist, deck) {
        Some(x) => {
            hist.retract();
            return x;
        },
        None => (),
    }

    // get relevant variables
    let info_set = hist.get_info_set(deck);
    let mut empty_node_info = NodeInfo::new();
    let node_info = node_map.entry(info_set).or_insert(empty_node_info);
    let current_player = hist.player_to_move;
    let opponent = other_player(current_player);
    let strategy = node_info.get_strategy(match current_player {
        Player::Player0 => prob_0,
        Player::Player1 => prob_1,
    });
    let mut utils: HashMap<Move, f64> = HashMap::new();
    let mut node_util = 0.0;

    // with each action, recursively call CFR with additional history and probability
    for m in MOVE_LIST {
        hist.append(opponent, m);
        let strat_m = strategy.get(&m).expect("Strategies should be exhaustive");
        // call cfr with additional history and probability
        let util_m = (-1.0) * match current_player {
            Player::Player0 => cfr_recursive(deck, node_map, hist, prob_0 * strat_m, prob_1),
            Player::Player1 => cfr_recursive(deck, node_map, hist, prob_0, prob_1 * strat_m),
        };
        hist.retract();
        // let util_m = 0.0;
        node_util += strat_m * util_m;
        utils.insert(m, util_m);
    }

    // for each action, compute and accumulate counterfactual regret
    for m in MOVE_LIST {
        let util_m = utils
            .get(&m)
            .expect("We should have inserted a utility for m in the previous for loop");
        let regret_m = util_m - node_util;
        let counterfact_prob = match current_player {
            Player::Player0 => prob_1,
            Player::Player1 => prob_0,
        };
        node_info.regret_sum.entry(m).and_modify(|r| {*r += counterfact_prob * regret_m});
    }

    node_util
}

// How to write the above code iterative-style:
// - represent nodes to visit as a vector. store them as "this many moves in, take this
//   action". then you don't have to store a ton of hist vectors.
//   - so instead of a "retract" method you have a "lop off at move n" method.
// - at each node, store the p0 and p1.
// - once you hit a terminal node, have a function that moves the utility backwards and
//   updates all the node utilities

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
