use colorful::{core::color_string::CString, Color, Colorful};
use std::{
    cmp::Ordering,
    collections::VecDeque,
    fmt::{Display, Formatter},
    ops::{Index, IndexMut},
};

use crate::{
    data::Data,
    search::{GamePlayer, SearchableGame, WinState},
};

const MAX_VALUE: i32 = 10;

// Optional rules
#[derive(Default, Clone, Debug)]
pub struct Rules {
    /// When a card is played, if two or more of the sides are touching other cards,
    /// and the ranks of those sides are identical to those on the sides of the card played,
    /// the other cards are flipped.
    pub same: bool,

    /// When a card is played, if two or more of the sides are touching other cards,
    /// and the sum of the ranks on the touching sides are identical to the sums on
    /// the other touching sides, the other cards are flipped.
    pub plus: bool,

    /// Cards must be played in the order they appear in the deck.
    pub order: bool,

    // Cards must be played in a random order determined before the match.
    pub chaos: bool, // TODO: ????? how to handle this

    /// Lower numbers flip higher numbers.
    pub reverse: bool,

    /// A rank 1 side can flip an A. When Reverse is in play, a rank A side will flip a 1.
    pub fallen_ace: bool,

    /// A card's ranks increase in proportion to the number of cards of the same type already played.
    pub ascension: bool,

    /// A card's ranks decrease in proportion to the number of cards of the same type already played.
    pub decension: bool,

    /// One random card from each player's deck will be swapped with the other.
    pub swap: bool, // TODO: ????? how to handle this
}
impl Rules {
    pub fn add_rule_from_csv(&mut self, rule: i32) {
        match rule {
            // 0: no rule
            // 1: roulette
            // 2: all open
            // 3: three open
            4 => self.same = true,
            // 5: sudden death
            6 => self.plus = true,
            // 7: random
            8 => self.order = true,
            9 => self.chaos = true,
            10 => self.reverse = true,
            11 => self.fallen_ace = true,
            12 => self.ascension = true,
            13 => self.decension = true,
            14 => self.swap = true,
            // 15: draft
            0 | 1 | 2 | 3 | 5 | 7 | 15 => {}
            _ => {
                println!("Warning: Found unknown rule {}", rule);
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum Player {
    Red,
    Blue,
}
impl Player {
    fn display_color(&self) -> Color {
        match *self {
            Player::Blue => Color::LightBlue,
            Player::Red => Color::LightRed,
        }
    }
}
impl Display for Player {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl GamePlayer for Player {
    fn other(&self) -> Self {
        match *self {
            Player::Blue => Player::Red,
            Player::Red => Player::Blue,
        }
    }
}
impl<T> Index<Player> for [T; 2] {
    type Output = T;

    fn index(&self, index: Player) -> &Self::Output {
        &self[index as usize]
    }
}
impl<T> IndexMut<Player> for [T; 2] {
    fn index_mut(&mut self, index: Player) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

#[derive(Copy, Clone)]
#[repr(usize)]
pub enum Direction {
    North,
    South,
    West,
    East,
}
impl Direction {
    fn opposite(&self) -> Direction {
        match *self {
            Direction::East => Direction::West,
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(usize)]
pub enum Suit {
    Primal,
    Beastman,
    Scion,
    Garlean,
}
impl Display for Suit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Suit::Primal => "P",
                Suit::Beastman => "B",
                Suit::Scion => "S",
                Suit::Garlean => "G",
            }
        )
    }
}
#[derive(Clone, Default)]
pub struct Modifiers([i32; 4]);
impl Index<Suit> for Modifiers {
    type Output = i32;

    fn index(&self, index: Suit) -> &Self::Output {
        &self.0[index as usize]
    }
}
impl IndexMut<Suit> for Modifiers {
    fn index_mut(&mut self, index: Suit) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

#[derive(Clone, Debug)]
pub struct Card {
    values: [i32; 4],
    pub suit: Option<Suit>, // TODO: have "None" suit instead of Option for better packing
}
impl Card {
    pub fn new(n: i32, s: i32, w: i32, e: i32, suit: Option<Suit>) -> Self {
        Card {
            values: [n, s, w, e],
            suit,
        }
    }

    /// Returns true if this card is beaten a card just played in the given position relative to this card.
    /// Example:
    ///     [this][other]
    /// would be represented by Position::East and
    ///     [other]
    ///     [this]
    /// would be represented by Position::North.
    fn is_flipped_by(
        &self,
        other: &Card,
        direction: Direction,
        modifiers: &Modifiers,
        rules: &Rules,
    ) -> bool {
        let my_value = self.get_modified_value(modifiers, direction);
        let other_value = other.get_modified_value(modifiers, direction.opposite());

        if !rules.reverse {
            if rules.fallen_ace && (my_value == MAX_VALUE) && (other_value == 1) {
                true
            } else {
                other_value > my_value
            }
        } else {
            if rules.fallen_ace && (my_value == 1) && (other_value == MAX_VALUE) {
                true
            } else {
                other_value < my_value
            }
        }
    }

    pub fn get_modified_value(&self, modifiers: &Modifiers, direction: Direction) -> i32 {
        self.values[direction as usize]
            + (self.suit.map(|s| modifiers[s]).unwrap_or(0))
                .min(MAX_VALUE)
                .max(0)
    }

    pub fn get_modified_value_display(
        &self,
        modifiers: &Modifiers,
        direction: Direction,
    ) -> String {
        let val = self.get_modified_value(modifiers, direction);
        if val >= MAX_VALUE {
            "A".to_string()
        } else {
            val.to_string()
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameMove {
    pub player: Player,
    pub card_idx: usize,
    pub placement: usize,
}

#[derive(Clone, Default)]
struct GameState {
    // 0, 1, 2
    // 3, 4, 5
    // 6, 7, 8
    board: [Option<(Card, Player)>; 9],
    hands: [[Option<(i32, Card)>; 10]; 2], // (id, card)
    modifiers: Modifiers,
    actual_hand_sizes: [usize; 2],
}
impl GameState {
    fn is_game_over(&self) -> bool {
        self.board.iter().all(|x| x.is_some())
    }

    fn scores(&self) -> [usize; 2] {
        let mut scores = self.actual_hand_sizes.clone();

        for space in &self.board {
            if let Some((_, player)) = space {
                scores[*player] += 1;
            }
        }

        scores
    }

    fn eval_position(&self, player: Player) -> f64 {
        let scores = self.scores();

        // If the game is over, then the score is either +/- infinity, or zero if there was a tie
        if self.is_game_over() {
            match scores[player].cmp(&scores[player.other()]) {
                Ordering::Greater => 100f64,
                Ordering::Equal => -30f64,
                Ordering::Less => -100f64,
            }
        } else {
            (scores[player] as f64) - (scores[player.other()] as f64)
        }
    }

    fn get_possible_moves(
        &self,
        player: Player,
        first_card_only: bool,
        result: &mut Vec<GameMove>,
    ) {
        result.clear();
        for candidate_position in 0..9 {
            if self.board[candidate_position].is_none() {
                'card_iter: for candidate_card in 0..self.hands[player].len() {
                    if self.hands[player][candidate_card].is_some() {
                        result.push(GameMove {
                            player,
                            card_idx: candidate_card,
                            placement: candidate_position,
                        });

                        if first_card_only {
                            break 'card_iter;
                        }
                    }
                }
            }
        }
    }
}

pub struct Game {
    // last entry is current state
    state_and_history: VecDeque<GameState>,
    rules: Rules,
    humans: [bool; 2],
}
impl Game {
    // Because of the order rule, it matters which player is human
    pub fn new(human_color: Player) -> Self {
        Game {
            state_and_history: {
                let mut history = VecDeque::with_capacity(100);
                history.push_back(Default::default());
                history
            },
            rules: Default::default(),
            humans: {
                let mut humans = [false; 2];
                humans[human_color] = true;
                humans
            },
        }
    }

    fn adjacency(from: usize, to: usize) -> Option<Direction> {
        Some(match (from, to) {
            (0, 1) | (1, 2) | (3, 4) | (4, 5) | (6, 7) | (7, 8) => Direction::East,
            (1, 0) | (2, 1) | (4, 3) | (5, 4) | (7, 6) | (8, 7) => Direction::West,
            (0, 3) | (1, 4) | (2, 5) | (3, 6) | (4, 7) | (5, 8) => Direction::South,
            (3, 0) | (4, 1) | (5, 2) | (6, 3) | (7, 4) | (8, 5) => Direction::North,
            _ => return None,
        })
    }

    fn current_state(&self) -> &GameState {
        self.state_and_history.back().unwrap()
    }

    // Note: directly modifies the current game state, doesn't affect history
    pub fn set_cards_in_hand(
        &mut self,
        player: Player,
        cards: &[(i32, Card); 5],
        actual_size: usize,
    ) {
        let state = self.state_and_history.back_mut().unwrap();
        let hand = &mut state.hands[player];

        for i in 0..5 {
            hand[i] = Some(cards[i].clone());
        }
        for i in 5..hand.len() {
            hand[i] = None;
        }

        state.actual_hand_sizes[player] = actual_size;
    }

    pub fn set_cards_for_npc(&mut self, player: Player, data: &Data, npc_name: &str) {
        let npc = data.npcs_by_name.get(npc_name).unwrap();
        let state = self.state_and_history.back_mut().unwrap();
        let hand = &mut state.hands[player];
        for i in 0..5 {
            if npc.fixed_cards[i] != 0 {
                hand[i] = Some((
                    npc.fixed_cards[i],
                    data.get_card(npc.fixed_cards[i]).unwrap().clone(),
                ));
            } else {
                hand[i] = None;
            }
        }

        for i in 0..5 {
            if npc.variable_cards[i] != 0 {
                hand[i + 5] = Some((
                    npc.variable_cards[i],
                    data.get_card(npc.variable_cards[i]).unwrap().clone(),
                ));
            } else {
                hand[i + 5] = None;
            }
        }

        state.actual_hand_sizes[player] = 5;
        self.rules = npc.rules.clone();
    }

    pub fn player_hand_card_name<'a, 'b>(
        &'a self,
        player: Player,
        idx: usize,
        data: &'b Data,
    ) -> &'b String {
        let id = self.current_state().hands[player][idx].as_ref().unwrap().0;
        data.card_names.get(&id).unwrap()
    }

    fn get_display(&self, pos: usize, dir: Direction) -> CString {
        let state = self.current_state();
        state.board[pos]
            .as_ref()
            .map(|(card, player)| {
                card.get_modified_value_display(&state.modifiers, dir)
                    .color(player.display_color())
            })
            .unwrap_or_else(|| " ".to_string().color(Color::Black))
    }

    fn get_hand_display(&self, player: Player) -> CString {
        self.current_state().actual_hand_sizes[player]
            .to_string()
            .color(player.display_color())
    }

    fn get_suit_display(&self, pos: usize) -> CString {
        self.current_state().board[pos]
            .as_ref()
            .map(|(card, player)| {
                card.suit
                    .map(|suit| suit.to_string().color(player.display_color()))
                    .unwrap_or_else(|| " ".color(player.display_color()))
            })
            .unwrap_or_else(|| " ".color(Color::Black))
    }
}
impl SearchableGame for Game {
    type Move = GameMove;
    type Player = Player;

    fn get_possible_moves(&self, player: Self::Player, buffer: &mut Vec<Self::Move>) {
        self.current_state().get_possible_moves(
            player,
            self.humans[player] && self.rules.order,
            buffer,
        )
    }

    fn evaluate_current_position_for(&self, player: Self::Player) -> f64 {
        self.current_state().eval_position(player)
    }

    fn apply_move(&mut self, mv: &Self::Move) {
        let mut new_state = self.current_state().clone();
        let (_, played_card) = new_state.hands[mv.player][mv.card_idx].take().unwrap();
        new_state.actual_hand_sizes[mv.player] -= 1;

        for possibly_adjacent in 0..9 {
            if let Some(direction) = Game::adjacency(possibly_adjacent, mv.placement) {
                if let Some((ref card, ref mut owner)) = new_state.board[possibly_adjacent] {
                    if card.is_flipped_by(
                        &played_card,
                        direction,
                        &new_state.modifiers,
                        &self.rules,
                    ) {
                        *owner = mv.player;
                    }
                }
            }
        }

        // TODO: handle SAME and PLUS rules

        if self.rules.ascension {
            if let Some(suit) = played_card.suit {
                new_state.modifiers[suit] += 1;
            }
        }
        if self.rules.decension {
            if let Some(suit) = played_card.suit {
                new_state.modifiers[suit] -= 1;
            }
        }

        new_state.board[mv.placement] = Some((played_card, mv.player));
        self.state_and_history.push_back(new_state);
    }

    fn undo_last_moves(&mut self, n: usize) {
        for _ in 0..n {
            self.state_and_history.pop_back();
        }
    }

    fn win_state(&self) -> WinState<Self> {
        let state = self.current_state();
        if state.is_game_over() {
            let scores = state.scores();
            match scores[Player::Red].cmp(&scores[Player::Blue]) {
                Ordering::Less => WinState::Winner(Player::Blue),
                Ordering::Equal => WinState::Tie,
                Ordering::Greater => WinState::Winner(Player::Red),
            }
        } else {
            WinState::NotFinished
        }
    }

    fn truncate_history_and_clone(&self) -> Self {
        Game {
            state_and_history: {
                let mut state = VecDeque::with_capacity(10);
                state.push_back(self.current_state().clone());
                state
            },
            rules: self.rules.clone(),
            humans: self.humans.clone(),
        }
    }
}
impl Display for Game {
    //   ┌─────┬─────┬─────┐
    //   │  0S │  0S │  0S │
    //   │ 0 0 │ 0 0 │ 0 0 │
    //   │  0  │  0  │  0  │
    //   ├─────┼─────┼─────┤
    //   │  0  │  0  │  0  │
    // 4 │ 0 0 │ 0 0 │ 0 0 │  5
    //   │  0  │  0  │  0  │
    //   ├─────┼─────┼─────┤
    //   │  0  │  0  │  0  │
    //   │ 0 0 │ 0 0 │ 0 0 │
    //   │  0  │  0  │  0  │
    //   └─────┴─────┴─────┘
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Direction::*;

        writeln!(f, "  ┌─────┬─────┬─────┐")?;
        writeln!(
            f,
            "  │  {}{} │  {}{} │  {}{} │",
            self.get_display(0, North),
            self.get_suit_display(0),
            self.get_display(1, North),
            self.get_suit_display(1),
            self.get_display(2, North),
            self.get_suit_display(2),
        )?;
        writeln!(
            f,
            "  │ {} {} │ {} {} │ {} {} │",
            self.get_display(0, West),
            self.get_display(0, East),
            self.get_display(1, West),
            self.get_display(1, East),
            self.get_display(2, West),
            self.get_display(2, East)
        )?;
        writeln!(
            f,
            "  │  {}  │  {}  │  {}  │",
            self.get_display(0, South),
            self.get_display(1, South),
            self.get_display(2, South)
        )?;
        writeln!(f, "  ├─────┼─────┼─────┤")?;
        writeln!(
            f,
            "  │  {}{} │  {}{} │  {}{} │",
            self.get_display(3, North),
            self.get_suit_display(3),
            self.get_display(4, North),
            self.get_suit_display(4),
            self.get_display(5, North),
            self.get_suit_display(5),
        )?;
        writeln!(
            f,
            "{} │ {} {} │ {} {} │ {} {} │ {}",
            self.get_hand_display(Player::Blue),
            self.get_display(3, West),
            self.get_display(3, East),
            self.get_display(4, West),
            self.get_display(4, East),
            self.get_display(5, West),
            self.get_display(5, East),
            self.get_hand_display(Player::Red),
        )?;
        writeln!(
            f,
            "  │  {}  │  {}  │  {}  │",
            self.get_display(3, South),
            self.get_display(4, South),
            self.get_display(5, South)
        )?;
        writeln!(f, "  ├─────┼─────┼─────┤")?;
        writeln!(
            f,
            "  │  {}{} │  {}{} │  {}{} │",
            self.get_display(6, North),
            self.get_suit_display(6),
            self.get_display(7, North),
            self.get_suit_display(7),
            self.get_display(8, North),
            self.get_suit_display(8),
        )?;
        writeln!(
            f,
            "  │ {} {} │ {} {} │ {} {} │",
            self.get_display(6, West),
            self.get_display(6, East),
            self.get_display(7, West),
            self.get_display(7, East),
            self.get_display(8, West),
            self.get_display(8, East)
        )?;
        writeln!(
            f,
            "  │  {}  │  {}  │  {}  │",
            self.get_display(6, South),
            self.get_display(7, South),
            self.get_display(8, South)
        )?;
        writeln!(f, "  └─────┴─────┴─────┘")?;

        Ok(())
    }
}
