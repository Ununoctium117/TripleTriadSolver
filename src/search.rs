use rand::{seq::SliceRandom, Rng};
use rayon::prelude::*;
use std::{cmp::Ordering, fmt::Debug, time::Instant};

const MONTE_CARLO_ITERATIONS: usize = 100_000;

pub trait GamePlayer: Copy + Clone + Debug + Send + Sync + Eq {
    fn other(&self) -> Self;
}

pub enum WinState<G: SearchableGame> {
    NotFinished,
    Tie,
    Winner(G::Player),
}

pub trait SearchableGame: Send + Sized {
    type Move: Debug + Sized + Send + Sync + Clone;
    type Player: GamePlayer;

    fn get_possible_moves(&self, player: Self::Player, buffer: &mut Vec<Self::Move>);
    fn evaluate_current_position_for(&self, player: Self::Player) -> f64;
    fn win_state(&self) -> WinState<Self>;
    fn truncate_history_and_clone(&self) -> Self;

    fn apply_move(&mut self, mv: &Self::Move);
    fn undo_last_moves(&mut self, n: usize);
}

pub fn get_best_move_for_player<G: SearchableGame>(
    game: &G,
    player: G::Player,
) -> (Option<G::Move>, (f64, Option<f64>)) {
    let mut game = game.truncate_history_and_clone();
    let alphabeta_start = Instant::now();
    let (best_moves, score) = alpha_beta(&mut game, 10, f64::NEG_INFINITY, f64::INFINITY, player);
    println!(
        "Found {} moves with best score {} (negamax duration: {:?})",
        best_moves.len(),
        score,
        Instant::now() - alphabeta_start
    );

    struct MoveSelection<G: SearchableGame> {
        mv: Option<G::Move>,
        win_ratio: f64,
    }
    fn no_move_selection<G: SearchableGame>() -> MoveSelection<G> {
        MoveSelection {
            mv: None,
            win_ratio: f64::NEG_INFINITY,
        }
    }
    fn combine_move_selection<G: SearchableGame>(
        sel1: MoveSelection<G>,
        sel2: MoveSelection<G>,
    ) -> MoveSelection<G> {
        if sel1.mv.is_none() {
            sel2
        } else if sel2.mv.is_none() {
            sel1
        } else if sel1.win_ratio > sel2.win_ratio {
            sel1
        } else {
            sel2
        }
    }

    match best_moves.len() {
        0 => (None, (score, None)),
        1 => (Some(best_moves[0].clone()), (score, None)),
        len => {
            println!("Entering Monte Carlo simulation with {} moves!", len);
            let monte_carlo_start = Instant::now();
            let best_best_move = best_moves
                .into_iter()
                .map(|mv| {
                    (
                        {
                            let mut game = game.truncate_history_and_clone();
                            game.apply_move(&mv);
                            game
                        },
                        mv,
                    )
                })
                .collect::<Vec<_>>()
                .into_par_iter()
                .map(move |(game, mv)| MoveSelection::<G> {
                    mv: Some(mv),
                    win_ratio: monte_carlo(game, player, MONTE_CARLO_ITERATIONS),
                })
                .reduce(no_move_selection, combine_move_selection);

            println!(
                "Monte carlo finished (duration: {:?})",
                Instant::now() - monte_carlo_start
            );
            (best_best_move.mv, (score, Some(best_best_move.win_ratio)))
        }
    }
}

// Evaluates the current game using a Monte-Carlo search (random moves), with "player" having just moved, and returns the fraction
// of games won by "player".
fn monte_carlo<G: SearchableGame>(mut game: G, player: G::Player, iterations: usize) -> f64 {
    let mut wins = 0;
    let mut ties = 0;

    let mut rng = rand::thread_rng();

    for _ in 0..iterations {
        match simulate_game_once(&mut game, player, &mut rng) {
            SimulationResult::PlayerWin => wins += 1,
            SimulationResult::Tie => ties += 1,
            SimulationResult::OpponentWin => {}
        }
    }

    // Ties count as 30% of a win
    // println!(
    //     "Monte Carlo simulation result: {} wins, {} ties, {} losses; ratio: {}",
    //     wins,
    //     ties,
    //     iterations - wins - ties,
    //     ((wins as f64) + (ties as f64 * 0.3)) / (iterations as f64)
    // );
    ((wins as f64) + (ties as f64 * 0.3)) / (iterations as f64)
}
enum SimulationResult {
    PlayerWin,
    Tie,
    OpponentWin,
}
fn simulate_game_once<G: SearchableGame>(
    game: &mut G,
    player: G::Player,
    rng: &mut impl Rng,
) -> SimulationResult {
    let mut moves_taken = 0;
    let mut current_player = player.other();

    let mut possible_moves = Vec::with_capacity(100);

    let result = loop {
        match game.win_state() {
            WinState::NotFinished => {}
            WinState::Tie => break SimulationResult::Tie,
            WinState::Winner(winner) if winner == player => {
                break SimulationResult::PlayerWin;
            }
            WinState::Winner(_) => {
                break SimulationResult::OpponentWin;
            }
        }

        possible_moves.clear();
        game.get_possible_moves(current_player, &mut possible_moves);
        let mv = possible_moves.choose(rng).unwrap();
        game.apply_move(mv);

        moves_taken += 1;
        current_player = current_player.other();
    };

    game.undo_last_moves(moves_taken);
    result
}

// Finds the best move for `player` given the current game state, with a maximum search depth.
// This is basically negamax search (TT is a zero sum game) with alpha-beta pruning.
fn alpha_beta<G: SearchableGame>(
    game: &mut G,
    depth: usize,
    mut alpha: f64,
    beta: f64,
    player: G::Player,
) -> (Vec<G::Move>, f64) {
    if depth == 0 {
        return (vec![], game.evaluate_current_position_for(player));
    }

    let mut possible_moves = Vec::with_capacity(10);
    game.get_possible_moves(player, &mut possible_moves);

    if possible_moves.is_empty() {
        return (vec![], game.evaluate_current_position_for(player));
    }

    let mut best_value = f64::NEG_INFINITY;
    let mut best_moves = Vec::with_capacity(100);

    for possible_move in possible_moves {
        game.apply_move(&possible_move);
        let (_, mut move_value) = alpha_beta(game, depth - 1, -beta, -alpha, player.other());
        move_value *= -1f64;
        game.undo_last_moves(1);

        match move_value.partial_cmp(&best_value) {
            Some(Ordering::Greater) => {
                best_value = move_value;
                best_moves.clear();
                best_moves.push(possible_move);
            }
            Some(Ordering::Equal) => {
                best_moves.push(possible_move);
            }
            _ => {}
        }

        alpha = alpha.max(best_value);
        if alpha >= beta {
            break;
        }
    }

    (best_moves, best_value)
}
