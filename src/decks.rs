use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, path::PathBuf};
use thiserror::Error;

use crate::{data::Data, game::Card};

#[derive(Debug, Error)]
pub enum SavedDeckError {
    #[error("Could not find config directory")]
    NoProjectDirs,

    #[error("Could not read/write config file")]
    IoError(#[from] std::io::Error),

    #[error("Could not parse config file")]
    SerdeErr(#[from] serde_json::Error),

    #[error("A saved deck has an unknown card")]
    UnknownCard,
}

#[derive(Serialize, Deserialize)]
pub struct SavedDecks {
    decks: HashMap<String, Deck>,

    #[serde(skip)]
    config_path: PathBuf,
}
impl SavedDecks {
    pub fn new() -> Result<Self, SavedDeckError> {
        let mut config_path =
            directories::ProjectDirs::from("com", "Ununoctium", "tripletriadsolver")
                .ok_or(SavedDeckError::NoProjectDirs)?
                .config_dir()
                .to_path_buf();
        config_path.push("decks.json");

        if config_path.exists() {
            let mut result: SavedDecks = serde_json::from_reader(File::open(&config_path)?)?;
            result.config_path = config_path;
            Ok(result)
        } else {
            std::fs::create_dir_all(config_path.parent().unwrap())?;
            let result = SavedDecks {
                decks: HashMap::new(),
                config_path,
            };
            result.save()?;
            Ok(result)
        }
    }

    pub fn add_deck(&mut self, name: String, cards: [String; 5]) -> Result<(), SavedDeckError> {
        self.decks.insert(
            name,
            Deck {
                created: Utc::now(),
                cards,
            },
        );
        self.save()?;
        Ok(())
    }

    pub fn remove_deck(&mut self, name: &str) -> Result<(), SavedDeckError> {
        self.decks.remove(name);
        self.save()?;
        Ok(())
    }

    pub fn get_deck(
        &self,
        name: &str,
        data: &Data,
    ) -> Result<([String; 5], [Card; 5]), SavedDeckError> {
        let deck = self.decks.get(name).unwrap();
        let cards = deck
            .cards
            .iter()
            .map(|card_name| {
                data.cards_by_name
                    .get(card_name)
                    .cloned()
                    .ok_or(SavedDeckError::UnknownCard)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok((deck.cards.clone(), cards.try_into().unwrap()))
    }

    pub fn get_deck_names(&self) -> Vec<String> {
        self.decks.keys().cloned().collect()
    }

    pub fn get_deck_count(&self) -> usize {
        self.decks.len()
    }

    fn save(&self) -> Result<(), SavedDeckError> {
        serde_json::to_writer_pretty(File::create(&self.config_path)?, self)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct Deck {
    created: DateTime<Utc>,
    cards: [String; 5],
}
