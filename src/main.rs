mod data;
mod game;
mod search;

use game::{Game, Player};
use search::SearchableGame;
use std::time::Instant;

use crate::search::GamePlayer;

fn main() {
    let loading_start = Instant::now();
    let data = data::load_all_data("data").unwrap();
    let loading_duration = Instant::now() - loading_start;
    println!("\nLoading data took {:?}", loading_duration);

    let mut game = Game::new(Player::Blue);
    game.set_cards_for_npc(Player::Blue, &data, "Isobe");
    game.set_cards_for_npc(Player::Red, &data, "Ogodei");

    println!("{}", game);

    let mut current_player = Player::Red;
    while !game.is_over() {
        println!("Finding {:?}'s best move...", current_player);
        let (best_move, score) = search::get_best_move_for_player(&mut game, current_player);

        println!(
            "{:?}'s best move: {:?} (score: {:?})",
            current_player, best_move, score
        );
        game.apply_move(&best_move.unwrap());

        println!("{}", game);
        current_player = current_player.other();
    }
}
