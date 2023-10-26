// Code to solve Kuhn Poker using Counterfactual Regret Minimization
// Following "An Introduction to Counterfactual Regret Minimization" by Neller and Lanctot (2013)

// stuff I actually want to do, in order:
// then do a post-flop solver (write tests for that)
// then do simple optimizations - stop iterating based on regret, weight early iterations less
// then maybe do abstract info sets
// then do a full poker solver (def with abstract info sets, might have to do it monte carlo)

use rand::Rng;
use std::collections::HashMap;
use std::time::SystemTime;

use crate::game::{Card, NUM_CARDS};
use crate::solver_tree::{ChancyHistory, Floating, InfoSet, NodeInfo, NodeUtils};

mod game;
mod solver_tree;

fn main() {
    let num_iters = 10_000;
    let mut rng = rand::thread_rng();
    let mut deck: [Card; NUM_CARDS] = [Card::Ace, Card::King, Card::Queen, Card::Jack, Card::Ten];

    let mut util = 0.0;
    let mut node_map: HashMap<InfoSet, NodeInfo> = HashMap::new();

    let start = SystemTime::now();

    for _ in 0..num_iters {
        shuffle_deck(&mut deck, &mut rng);
        util += cfr(&deck, &mut node_map);
    }

    println!("Average game value is {}", util / (num_iters as Floating));
    for (info_set, node_info) in node_map.into_iter() {
        if !info_set.is_terminal() {
            let legal_moves = info_set.get_next_moves();
            let avg_strategy = node_info.get_average_strategy(&legal_moves);
            println!(
                "At info_set {:?}, avg strategy is {:?}",
                info_set, avg_strategy
            );
        }
    }

    match start.elapsed() {
        Ok(elapsed) => {
            println!("Time elapsed: {} ms", elapsed.as_millis());
        }
        Err(e) => {
            println!("Timing error: {:?}", e);
        }
    }

    println!("Number of iterations: {}", num_iters);
}

fn shuffle_deck(deck: &mut [Card; NUM_CARDS], rng: &mut rand::rngs::ThreadRng) {
    for i in (1..NUM_CARDS).rev() {
        let j = rng.gen_range(0..(i + 1));
        deck.swap(i, j);
    }
}

fn cfr(deck: &[Card; NUM_CARDS], node_map: &mut HashMap<InfoSet, NodeInfo>) -> Floating {
    let mut utils_map: HashMap<InfoSet, NodeUtils> = HashMap::new();

    // Get a topological ordering of the game tree
    let top_order = get_topological_ordering(deck, node_map);

    // Iterate thru nodes in reverse topological order, so we can propagate values up the tree.
    for chancy_hist in top_order.into_iter().rev() {
        let info_set = chancy_hist.to_info_set(deck);
        let node_info = node_map
            .get_mut(&info_set)
            .expect("Entries should have been added to node map during topological sort");
        utils_map.insert(
            chancy_hist.to_info_set(deck),
            NodeUtils::new(&info_set.get_next_moves()),
        );

        if let Some(u) = chancy_hist.util_if_terminal(deck) {
            // chancy_hist is terminal
            // No need to set move utils here
            // but we do need to say what the value of the node is for backwards induction.
            let node_utils = utils_map.get_mut(&info_set).unwrap();
            node_utils.value = u;
            // No need to calculate counterfactual regrets or update strategies for a terminal node.
        } else {
            // First, set move utilities by the values of the successor nodes
            let legal_moves = info_set.get_next_moves();
            for m in &legal_moves {
                let prob_move = node_info.get_strategy(*m);
                let next_chancy_hist = chancy_hist.extend(*m, prob_move).unwrap();
                let next_info_set = next_chancy_hist.to_info_set(deck);
                let next_node_value = utils_map.get(&next_info_set).expect("Utils should have been set earlier in the loop, because we're iterating thru the reverse of a topological sort").value;
                let node_utils = utils_map
                    .get_mut(&info_set)
                    .expect("We should have created this entry at the start of this loop");
                node_utils.move_utils.insert(*m, (-1.0) * next_node_value);
                node_utils.value += prob_move * (-1.0) * next_node_value;
            }

            // Next, calculate counterfactual regrets and update the regret sums.
            let node_utils = utils_map
                .get_mut(&info_set)
                .expect("We should have created this entry at the start of this loop");
            for m in &legal_moves {
                let util_m = node_utils
                    .move_utils
                    .get(m)
                    .expect("We should have just calculated utils for all moves");
                let regret_m = util_m - node_utils.value;
                let counterfact_prob = chancy_hist.get_counterfactual_reach_prob();
                node_info.update_regret(*m, counterfact_prob * regret_m);
            }

            // Finally, update strategies.
            let reach_prob = chancy_hist.get_reach_prob();
            node_info.update_strategy(&legal_moves, reach_prob);
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
    info_set: &InfoSet,
    node_info: &NodeInfo,
    node_stack: &mut Vec<ChancyHistory>,
) {
    let legal_moves = info_set.get_next_moves();
    for m in legal_moves {
        // get prob of taking m from this info set
        let prob_move = node_info.get_strategy(m);
        let next_chancy_hist = chancy_hist.extend(m, prob_move).unwrap();
        node_stack.push(next_chancy_hist);
    }
}

fn get_topological_ordering(
    deck: &[Card; NUM_CARDS],
    node_map: &mut HashMap<InfoSet, NodeInfo>,
) -> Vec<ChancyHistory> {
    let mut unseen_nodes = vec![ChancyHistory::new()];
    let mut ordered_nodes: Vec<ChancyHistory> = Vec::new();

    while let Some(chancy_hist) = unseen_nodes.pop() {
        let info_set = chancy_hist.to_info_set(deck);
        let legal_moves = info_set.get_next_moves();
        let node_info = node_map
            .entry(info_set.clone())
            .or_insert(NodeInfo::new(&legal_moves));
        if !chancy_hist.is_terminal() {
            append_children_to_stack(&chancy_hist, &info_set, node_info, &mut unseen_nodes);
        }
        ordered_nodes.push(chancy_hist);
    }

    ordered_nodes
}
