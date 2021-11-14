mod data;
mod decks;
mod game;
mod search;

use data::Data;
use decks::SavedDecks;
use directories::ProjectDirs;
use game::{Card, Direction, Game, GameMove, Modifiers, Player};
use inquire::{Confirm, Select, Text};
use search::{GamePlayer, SearchableGame, WinState};
use std::{
    cmp::Ordering,
    collections::HashSet,
    fmt::{Display, Formatter},
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

#[derive(Clone, PartialEq, Eq)]
struct PossibleCard<'a> {
    name: &'a str,
    id: i32,
}
impl<'a> Display for PossibleCard<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
impl<'a> PartialOrd for PossibleCard<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.id.partial_cmp(&other.id)
    }
}
impl<'a> Ord for PossibleCard<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

fn register_deck(data: &data::Data, saved_decks: &mut SavedDecks) {
    // Get deck name
    let name = Text::new("Deck name:").prompt().unwrap();

    let mut cards: Vec<PossibleCard> = data
        .card_names
        .iter()
        .map(|(id, name)| PossibleCard { id: *id, name })
        .collect();
    cards.sort();

    println!("\nReminder: deck order matters!\n");
    let deck_card_names = [
        "First card:",
        "Second card:",
        "Third card:",
        "Fourth card:",
        "Fifth card:",
    ]
    .map(|prompt| Select::new(prompt, cards.clone()).prompt().unwrap());

    saved_decks
        .add_deck(name, deck_card_names.map(|c| c.id))
        .unwrap();
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
                write!(f, "{}. {}", idx + 2, name)
            }
        }
    }
}
fn view_decks(data: &Data, saved_decks: &SavedDecks) {
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
        match Select::new("Which deck?", options.clone())
            .prompt()
            .unwrap()
        {
            ViewDeckOption::GoBack => return,
            ViewDeckOption::ViewCards(name, _) => {
                print_deck(
                    &saved_decks.get_deck(&name).unwrap().map(|id| Some(id)),
                    data,
                );
            }
        }
    }
}

fn get_padding(name: &str) -> (usize, usize) {
    let padding = (name.len() + 2).saturating_sub(5);
    (
        padding / 2,
        if padding % 2 == 0 {
            padding / 2
        } else {
            (padding / 2) + 1
        },
    )
}

fn print_deck(deck: &[Option<i32>; 5], data: &Data) {
    let modifiers = Modifiers::default();

    let deck: [Option<(&str, &Card)>; 5] = deck.map(|id| {
        id.map(|id| {
            (
                data.card_names.get(&id).unwrap().as_str(),
                data.get_card(id).unwrap(),
            )
        })
    });

    let mut top_row = String::from("┌ ");
    top_row.push_str(
        &(0..5)
            .map(|i| {
                deck[i]
                    .map(|(name, _)| {
                        let mut title = name.to_string();
                        title.push_str(&" ".repeat(4usize.saturating_sub(name.len())));
                        title
                    })
                    .unwrap_or_else(|| "─────".to_string())
            })
            .collect::<Vec<_>>()
            .join(" ┬ "),
    );
    top_row.push_str(" ┐");

    let mut north_row = String::from("│");
    north_row.push_str(
        &(0..5)
            .map(|i| {
                deck[i]
                    .as_ref()
                    .map(|(name, card)| {
                        let (l, r) = get_padding(name);
                        format!(
                            "{}  {}{} {}",
                            " ".repeat(l),
                            card.get_modified_value(&modifiers, Direction::North),
                            card.suit
                                .map(|suit| suit.to_string())
                                .unwrap_or_else(|| " ".to_string()),
                            " ".repeat(r),
                        )
                    })
                    .unwrap_or_else(|| "     ".to_string())
            })
            .collect::<Vec<_>>()
            .join("│"),
    );
    north_row.push('│');

    let mut mid_row = String::from("│");
    mid_row.push_str(
        &(0..5)
            .map(|i| {
                deck[i]
                    .as_ref()
                    .map(|(name, card)| {
                        let (l, r) = get_padding(name);
                        format!(
                            "{} {} {} {}",
                            " ".repeat(l),
                            card.get_modified_value_display(&modifiers, Direction::West),
                            card.get_modified_value_display(&modifiers, Direction::East),
                            " ".repeat(r),
                        )
                    })
                    .unwrap_or_else(|| "     ".to_string())
            })
            .collect::<Vec<_>>()
            .join("│"),
    );
    mid_row.push('│');

    let mut south_row = String::from("│");
    south_row.push_str(
        &(0..5)
            .map(|i| {
                deck[i]
                    .as_ref()
                    .map(|(name, card)| {
                        let (l, r) = get_padding(name);
                        format!(
                            "{}  {}  {}",
                            " ".repeat(l),
                            card.get_modified_value(&modifiers, Direction::South),
                            " ".repeat(r),
                        )
                    })
                    .unwrap_or_else(|| "     ".to_string())
            })
            .collect::<Vec<_>>()
            .join("│"),
    );
    south_row.push('│');

    let mut bottom_row = String::from("└─");
    bottom_row.push_str(
        &(0..5)
            .map(|i| {
                deck[i]
                    .as_ref()
                    .map(|(name, _)| "─".repeat(name.len().max(4)))
                    .unwrap_or_else(|| "─────".to_string())
            })
            .collect::<Vec<_>>()
            .join("─┴─"),
    );
    bottom_row.push_str("─┘");

    println!("{}", top_row);
    println!("{}", north_row);
    println!("{}", mid_row);
    println!("{}", south_row);
    println!("{}", bottom_row);
}
struct PossiblePlacement(usize);
impl Display for PossiblePlacement {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self.0 {
                0 => "NW",
                1 => "N",
                2 => "NE",
                3 => "W",
                4 => "Center",
                5 => "E",
                6 => "SW",
                7 => "S",
                8 => "SE",
                _ => unreachable!(),
            }
        )
    }
}

fn pick_move(moves: &[GameMove], game: &Game, data: &Data) -> usize {
    struct PossibleCard<'a> {
        card_idx: usize,
        name: &'a String,
    }
    impl<'a> Display for PossibleCard<'a> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.name)
        }
    }

    let possible_cards = moves
        .iter()
        .map(|mv| (mv.player, mv.card_idx))
        .collect::<HashSet<_>>()
        .iter()
        .map(|(player, card_idx)| PossibleCard {
            card_idx: *card_idx,
            name: game.player_hand_card_name(*player, *card_idx, data),
        })
        .collect::<Vec<_>>();

    let card_selection = Select::new("What card?", possible_cards).prompt().unwrap();

    let possible_positions = moves
        .iter()
        .filter(|mv| mv.card_idx == card_selection.card_idx)
        .map(|mv| PossiblePlacement(mv.placement))
        .collect();

    let pos_selection = Select::new("Where?", possible_positions).prompt().unwrap();

    moves
        .iter()
        .enumerate()
        .find(|(_, mv)| mv.card_idx == card_selection.card_idx && mv.placement == pos_selection.0)
        .unwrap()
        .0
}

fn vs_npc(data: &Data, saved_decks: &SavedDecks) {
    if saved_decks.get_deck_count() == 0 {
        println!("You must have at least 1 registered deck to play an NPC!");
        return;
    }

    let mut ordered_names = data.npcs_by_name.keys().collect::<Vec<_>>();
    ordered_names.sort();
    let npc_name = Select::new("Which NPC?", ordered_names).prompt().unwrap();

    let mut deck_names = saved_decks.get_deck_names();
    deck_names.sort();
    let deck = Select::new("Which deck are you using?", deck_names)
        .prompt()
        .unwrap();

    let deck = saved_decks.get_deck(&deck).unwrap();

    let mut current_player = Select::new("Who goes first?", vec![Player::Blue, Player::Red])
        .prompt()
        .unwrap();
    let mut possible_moves = Vec::with_capacity(100);

    let mut game = Game::new(Player::Blue); // Human is always Blue vs NPCs
    game.set_cards_in_hand(
        Player::Blue,
        &deck
            .clone()
            .map(|id| (id, data.get_card(id).unwrap().clone())),
        5,
    );
    game.set_cards_for_npc(Player::Red, data, npc_name);

    let result = loop {
        match game.win_state() {
            WinState::NotFinished => {}
            WinState::Tie => break "Tie!",
            WinState::Winner(Player::Blue) => break "You win!",
            WinState::Winner(Player::Red) => break "You lose!",
        }

        println!("{}", game);

        game.get_possible_moves(current_player, &mut possible_moves);

        let move_sel = match current_player {
            Player::Red => {
                println!("What did the NPC do?");
                pick_move(&possible_moves, &game, data)
            }
            Player::Blue => {
                println!("Finding optimal move...");

                let (recommended_move, (score, _)) =
                    search::get_best_move_for_player(&game, current_player);

                let recommended_move = recommended_move.unwrap();

                println!(
                    "Recommended move: Play your {} card in the {}. (Score: {})",
                    game.player_hand_card_name(current_player, recommended_move.card_idx, data),
                    PossiblePlacement(recommended_move.placement),
                    score
                );

                println!("What did you actually do?");
                pick_move(&possible_moves, &game, data)
            }
        };

        game.apply_move(&possible_moves[move_sel]);
        current_player = current_player.other();
    };

    println!("Game finished! Result: {}", result);
}

fn main() {
    let project_dirs = ProjectDirs::from("com", "ununoctium", "TripleTriadSolver").unwrap();

    let data = data::Data::new(&project_dirs).unwrap();
    let mut saved_decks = SavedDecks::new(&project_dirs).unwrap();

    println!();

    loop {
        // Get user input
        println!(
            "You have {} registered decks.",
            saved_decks.get_deck_count()
        );
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

        println!();
    }
}
