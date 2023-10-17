// Code to solve Rock Paper Scissors using Counterfactual Regret Minimization
// Following "An Introduction to Counterfactual Regret Minimization" by Neller and Lanctot (2013)

// TODO document some stuff

use rand::Rng;
use std::collections::HashMap;

use crate::game::{Card, MOVE_LIST, NUM_CARDS};
use crate::solver_tree::{ChancyHistory, Floating, InfoSet, NodeInfo, NodeUtils};

mod game;
mod solver_tree;

fn main() {
    let num_iters = 1_000;
    let mut rng = rand::thread_rng();
    let mut deck: [Card; NUM_CARDS] = [Card::Ace, Card::King, Card::Queen];

    let mut util = 0.0;
    let mut node_map: HashMap<InfoSet, NodeInfo> = HashMap::new();

    for _ in 0..num_iters {
        shuffle_deck(&mut deck, &mut rng);
        util += cfr(&deck, &mut node_map);
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

fn cfr(deck: &[Card; NUM_CARDS], node_map: &mut HashMap<InfoSet, NodeInfo>) -> Floating {
    let mut node_stack: Vec<ChancyHistory> = vec![ChancyHistory::new()];
    let mut utils_map: HashMap<InfoSet, NodeUtils> = HashMap::new();

    // search the game tree depth-first until finding terminal nodes
    // then go back up the tree updating the utilities of each node and action
    while !node_stack.is_empty() {
        let chancy_hist = node_stack
            .pop()
            .expect("Node stack should be non-empty at the start of this loop");
        let option_util = chancy_hist.util_if_terminal(deck);
        match option_util {
            Some(utility) => {
                // our node is terminal
                // update each node's value and utility of each action
                update_utils(&chancy_hist, deck, node_map, &mut utils_map, utility);
            }
            None => {
                // our node is not terminal
                // so add child nodes to the node stack
                let info_set = chancy_hist.to_info_set(deck);
                let node_info = node_map.entry(info_set).or_insert(NodeInfo::new());
                append_children_to_stack(&chancy_hist, &node_info, &mut node_stack);
            }
        }
    }

    // search the game tree again to calculate counterfactual regrets, now that utilities are
    // calculated
    node_stack.push(ChancyHistory::new());
    while !node_stack.is_empty() {
        // get a node
        let chancy_hist = node_stack
            .pop()
            .expect("Node stack should be non-empty at the start of this loop");
        let info_set = chancy_hist.to_info_set(deck);
        let node_info = node_map
            .get(&info_set)
            .expect("Info entries were added to all nodes in the last traversal");

        // append children to stack before we start updating move probabilities
        append_children_to_stack(&chancy_hist, &node_info, &mut node_stack);

        // calculate the regret of each action

        // push those to update the cumulative counterfactual regret
        // (by multiplying the regrets with the counterfactual arrival probabilities)
        // (while doing that, secretly also update the strategies and strategy sums)
    }

    // return the utility of the start node
    let start_node = ChancyHistory::new();
    utils_map
        .get(&start_node.to_info_set(deck))
        .expect("We should have calculated info for this node in the main loop")
        .value
}

fn append_children_to_stack(
    chancy_hist: &ChancyHistory,
    node_info: &NodeInfo,
    node_stack: &mut Vec<ChancyHistory>,
) {
    for m in MOVE_LIST {
        // get prob of taking m from this info set
        let prob_move = node_info.get_strategy(m);
        let next_chancy_hist = chancy_hist.extend(m, prob_move);
        node_stack.push(next_chancy_hist);
    }
}

fn update_utils(
    chancy_hist: &ChancyHistory,
    deck: &[Card; NUM_CARDS],
    node_map: &mut HashMap<InfoSet, NodeInfo>,
    utils_map: &mut HashMap<InfoSet, NodeUtils>,
    terminal_utility: Floating,
) {
    let mut player_utility: Floating = terminal_utility;
    let mut reach_prob: Floating = 1.0;
    // iterate over non-terminal prefixes of this history, from the end to the start
    for n in (0..chancy_hist.len() - 1).rev() {
        let (info_set, m) = chancy_hist.truncate(n, deck);
        // we've switched players, so utility has changed sign
        player_utility *= -1.0;
        // getting node info and node utils
        let node_info = node_map.get(&info_set).expect(
            "We should have reached non-terminal nodes earlier in DFS and made node infos for them."
        );
        // getting them in this order because I want to rarely clone info_set,
        // and node_info should actually always be full of entries
        // because we create it if needed when handling non-terminal nodes
        // let node_info = node_map.get(&info_set) {
        //     None => node_map.entry(info_set.clone()).or_insert(NodeInfo::new()),
        //     Some(n_info) => n_info,
        // };
        let node_utils = utils_map.entry(info_set).or_insert(NodeUtils::new());
        // add the discounted utility to the node value
        node_utils
            .move_utils
            .entry(m)
            .and_modify(|u| *u += reach_prob * player_utility);
        // update the node value by its discounted utility,
        // updating the discount by the probability we take the action in question
        let prob_next_move = node_info.get_strategy(m);
        reach_prob *= prob_next_move;
        node_utils.value += reach_prob * player_utility;
    }
}

// fn cfr_recursive(
//     deck: &[Card; NUM_CARDS],
//     node_map: &mut HashMap<InfoSet, NodeInfo>,
//     hist: &mut History,
//     prob_0: Floating,
//     prob_1: Floating,
// ) -> Floating {
//     // TODO: re-write it in DFS style to not be recursive
//     // TODO: write check that the length of hist is the same at the start and end of this function body

//     // return utility of terminal nodes
//     match util_if_terminal(hist, deck) {
//         Some(x) => {
//             hist.retract();
//             return x;
//         },
//         None => (),
//     }

//     // get relevant variables
//     let info_set = hist.get_info_set(deck);
//     let mut empty_node_info = NodeInfo::new();
//     let node_info = node_map.entry(info_set).or_insert(empty_node_info);
//     let current_player = hist.player_to_move;
//     let opponent = other_player(current_player);
//     let strategy = node_info.get_strategy(match current_player {
//         Player::Player0 => prob_0,
//         Player::Player1 => prob_1,
//     });
//     let mut utils: HashMap<Move, Floating> = HashMap::new();
//     let mut node_util = 0.0;

//     // with each action, recursively call CFR with additional history and probability
//     for m in MOVE_LIST {
//         hist.append(opponent, m);
//         let strat_m = strategy.get(&m).expect("Strategies should be exhaustive");
//         // call cfr with additional history and probability
//         // let util_m = (-1.0) * match current_player {
//         //     Player::Player0 => cfr_recursive(deck, node_map, hist, prob_0 * strat_m, prob_1),
//         //     Player::Player1 => cfr_recursive(deck, node_map, hist, prob_0, prob_1 * strat_m),
//         // };
//         hist.retract();
//         let util_m = 0.0;
//         node_util += strat_m * util_m;
//         utils.insert(m, util_m);
//     }

//     // for each action, compute and accumulate counterfactual regret
//     for m in MOVE_LIST {
//         let util_m = utils
//             .get(&m)
//             .expect("We should have inserted a utility for m in the previous for loop");
//         let regret_m = util_m - node_util;
//         let counterfact_prob = match current_player {
//             Player::Player0 => prob_1,
//             Player::Player1 => prob_0,
//         };
//         node_info.regret_sum.entry(m).and_modify(|r| {*r += counterfact_prob * regret_m});
//     }

//     node_util
// }

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
