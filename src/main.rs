mod data;
mod decks;
mod game;
mod search;

use data::Data;
use game::Card;
use inquire::{Confirm, Select, Text};
use std::{
    fmt::{Display, Formatter},
    time::Instant,
};

use crate::{
    decks::SavedDecks,
    game::{Direction, Modifiers},
};

enum UserAction {
    PlayVsNpc,
    RegisterDeck,
    DeleteDeck,
    ViewDecks,
    Quit,
}
impl Display for UserAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                UserAction::PlayVsNpc => "1. Play against an NPC",
                UserAction::RegisterDeck => "2. Register a deck",
                UserAction::ViewDecks => "3. View your registered decks",
                UserAction::DeleteDeck => "4. Delete a registered deck",
                UserAction::Quit => "5. Quit",
            }
        )
    }
}

fn register_deck(data: &data::Data, saved_decks: &mut SavedDecks) {
    // Get deck name
    let name = Text::new("Deck name:").prompt().unwrap();

    println!("\nReminder: deck order matters!\n");
    let deck_card_names = [
        "First card:",
        "Second card:",
        "Third card:",
        "Fourth card:",
        "Fifth card:",
    ]
    .map(|prompt| {
        Select::new(prompt, data.ordered_card_names.clone())
            .prompt()
            .unwrap()
    });

    saved_decks.add_deck(name, deck_card_names).unwrap();
    println!("Deck saved!\n");
}

enum DeckDeleteOption {
    Cancel,
    Delete(String, usize),
}
impl Display for DeckDeleteOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            DeckDeleteOption::Cancel => write!(f, "1. Cancel"),
            DeckDeleteOption::Delete(ref name, idx) => write!(f, "{}. Deck: {}", idx + 2, name),
        }
    }
}
fn delete_deck(saved_decks: &mut SavedDecks) {
    let options = std::iter::once(DeckDeleteOption::Cancel)
        .chain(
            saved_decks
                .get_deck_names()
                .into_iter()
                .enumerate()
                .map(|(i, name)| DeckDeleteOption::Delete(name, i)),
        )
        .collect();
    match Select::new("Which deck would you like to delete?", options)
        .prompt()
        .unwrap()
    {
        DeckDeleteOption::Cancel => println!("Cancelled.\n"),
        DeckDeleteOption::Delete(name, _) => {
            if Confirm::new("Are you sure?").prompt().unwrap() {
                saved_decks.remove_deck(&name).unwrap();
                println!("{} deleted.", name);
            } else {
                println!("Cancelled.\n");
            }
        }
    };
}

#[derive(Clone)]
enum ViewDeckOption {
    GoBack,
    ViewCards(String, usize),
}
impl Display for ViewDeckOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match *self {
            ViewDeckOption::GoBack => write!(f, "1. Go back"),
            ViewDeckOption::ViewCards(ref name, idx) => {
                write!(f, "{}. View cards in deck {}", idx + 2, name)
            }
        }
    }
}
fn view_decks(data: &Data, saved_decks: &SavedDecks) {
    println!("Your saved decks:");
    println!("{:?}\n", saved_decks.get_deck_names());

    let options = std::iter::once(ViewDeckOption::GoBack)
        .chain(
            saved_decks
                .get_deck_names()
                .into_iter()
                .enumerate()
                .map(|(i, name)| ViewDeckOption::ViewCards(name, i)),
        )
        .collect::<Vec<_>>();

    loop {
        match Select::new("What would you like to do?", options.clone())
            .prompt()
            .unwrap()
        {
            ViewDeckOption::GoBack => return,
            ViewDeckOption::ViewCards(name, _) => {
                let (names, cards) = saved_decks.get_deck(&name, data).unwrap();
                print_deck(names, cards);
            }
        }
    }
}

fn print_deck(names: [String; 5], deck: [Card; 5]) {
    let modifiers = Modifiers::default();
    for i in 0..5 {
        println!("{}", names[i]);
        println!("┌─────┐");
        println!(
            "│  {}{} │",
            deck[i].get_modified_value(&modifiers, Direction::North),
            deck[i]
                .suit
                .map(|s| s.to_string())
                .unwrap_or_else(|| " ".to_string())
        );
        println!(
            "│ {} {} │",
            deck[i].get_modified_value(&modifiers, Direction::West),
            deck[i].get_modified_value(&modifiers, Direction::East)
        );
        println!(
            "│  {}  │",
            deck[i].get_modified_value(&modifiers, Direction::South)
        );
        println!("└─────┘\n");
    }
}

fn vs_npc(data: &Data, saved_decks: &SavedDecks) {
    todo!()
}

fn main() {
    let loading_start = Instant::now();
    let data = data::load_all_data("data").unwrap();
    let loading_duration = Instant::now() - loading_start;
    println!("\nLoaded card and NPC data in {:?}\n", loading_duration);

    let mut saved_decks = SavedDecks::new().unwrap();

    loop {
        // Get user input
        match Select::new(
            "What would you like to do?",
            vec![
                UserAction::PlayVsNpc,
                UserAction::RegisterDeck,
                UserAction::ViewDecks,
                UserAction::DeleteDeck,
                UserAction::Quit,
            ],
        )
        .prompt()
        .unwrap()
        {
            UserAction::PlayVsNpc => vs_npc(&data, &saved_decks),
            UserAction::RegisterDeck => register_deck(&data, &mut saved_decks),
            UserAction::DeleteDeck => delete_deck(&mut saved_decks),
            UserAction::ViewDecks => view_decks(&data, &saved_decks),
            UserAction::Quit => return,
        }
    }

    // let mut game = Game::new(Player::Blue);
    // game.set_cards_for_npc(Player::Blue, &data, "Isobe");
    // game.set_cards_for_npc(Player::Red, &data, "Ogodei");

    // println!("{}", game);

    // let mut current_player = Player::Red;
    // while !game.is_over() {
    //     println!("Finding {:?}'s best move...", current_player);
    //     let (best_move, score) = search::get_best_move_for_player(&mut game, current_player);

    //     println!(
    //         "{:?}'s best move: {:?} (score: {:?})",
    //         current_player, best_move, score
    //     );
    //     game.apply_move(&best_move.unwrap());

    //     println!("{}", game);
    //     current_player = current_player.other();
    // }
}
