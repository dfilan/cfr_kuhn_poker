// Code to solve Kuhn Poker using Counterfactual Regret Minimization
// Following "An Introduction to Counterfactual Regret Minimization" by Neller and Lanctot (2013)

// TODO document some stuff
// TODO: think about whether we can prune subtrees???

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

    println!("Average game value is {}", util / (num_iters as Floating));
    for (info_set, node_info) in node_map.into_iter() {
        let avg_strategy = node_info.get_average_strategy();
        println!(
            "At info_set {:?}, avg strategy is {:?}",
            info_set, avg_strategy
        );
    }
}

fn shuffle_deck(deck: &mut [Card; NUM_CARDS], rng: &mut rand::rngs::ThreadRng) {
    for i in (1..NUM_CARDS).rev() {
        let j = rng.gen_range(0..(i + 1));
        deck.swap(i, j);
    }
}

fn cfr(deck: &[Card; NUM_CARDS], node_map: &mut HashMap<InfoSet, NodeInfo>) -> Floating {
    let mut node_stack: Vec<ChancyHistory> = vec![ChancyHistory::new()];
    let mut utils_map: HashMap<InfoSet, NodeUtils> = HashMap::new();

    // search the game tree depth-first until finding terminal nodes
    // then go back up the tree updating the utilities of each node and action
    while let Some(chancy_hist) = node_stack.pop() {
        let option_util = chancy_hist.util_if_terminal(deck);
        match option_util {
            Some(utility) => {
                // our node is terminal
                // update each node's value and utility of each action
                update_utils(&chancy_hist, deck, node_map, &mut utils_map, utility);
            }
            None => {
                // our node is not terminal
                // add child nodes to the node stack
                let info_set = chancy_hist.to_info_set(deck);
                let node_info = node_map.entry(info_set).or_insert(NodeInfo::new());
                append_children_to_stack(&chancy_hist, node_info, &mut node_stack);
            }
        }
    }

    // search the game tree again to calculate counterfactual regrets, now that utilities are
    // calculated
    node_stack.push(ChancyHistory::new());
    while let Some(chancy_hist) = node_stack.pop() {
        if chancy_hist.util_if_terminal(deck).is_none() {
            // node isn't terminal
            let info_set = chancy_hist.to_info_set(deck);
            let node_info = node_map
                .get_mut(&info_set)
                .expect("Info entries were added to all nodes in the last traversal");
            let node_utils = utils_map
                .get(&info_set)
                .expect("Utilities were added to all nodes in the last traversal");

            // append children to stack before we start updating move probabilities
            append_children_to_stack(&chancy_hist, node_info, &mut node_stack);

            let node_value = node_utils.value;
            // calculate the regret of each action, and update the cumulative counterfactual regret
            for m in MOVE_LIST {
                let util_m = node_utils
                    .move_utils
                    .get(&m)
                    .expect("We should have calculated utils for all moves");
                let regret_m = util_m - node_value;
                let counterfact_prob = chancy_hist.get_counterfactual_reach_prob();
                // update node info with the counterfactual regret
                node_info.update_regret(m, counterfact_prob * regret_m);
            }

            // then update the strategies and strategy sums
            let reach_prob = chancy_hist.get_reach_prob();
            node_info.update_strategy(reach_prob);
        }
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
    node_map: &HashMap<InfoSet, NodeInfo>,
    utils_map: &mut HashMap<InfoSet, NodeUtils>,
    terminal_utility: Floating,
) {
    let mut player_utility: Floating = terminal_utility;
    let mut reach_prob: Floating = 1.0;
    // iterate over non-terminal prefixes of this history, from the end to the start
    for n in (0..chancy_hist.len()).rev() {
        let (info_set, m) = chancy_hist.truncate(n, deck);
        // we've switched players, so utility has changed sign
        player_utility *= -1.0;
        // getting node info and node utils
        let node_info = node_map.get(&info_set).expect(
            "We should have reached non-terminal nodes earlier in DFS and made node infos for them."
        );
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
