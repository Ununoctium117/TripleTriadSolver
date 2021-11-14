use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SavedDeckError {
    #[error("Could not read/write config file")]
    IoError(#[from] std::io::Error),

    #[error("Could not parse config file")]
    SerdeError(#[from] serde_json::Error),
}

#[derive(Serialize, Deserialize)]
pub struct SavedDecks {
    decks: HashMap<String, Deck>,

    #[serde(skip)]
    config_path: PathBuf,
}
impl SavedDecks {
    pub fn new(project_dirs: &ProjectDirs) -> Result<Self, SavedDeckError> {
        let mut config_path = project_dirs.config_dir().to_path_buf();
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

    pub fn add_deck(&mut self, name: String, cards: [i32; 5]) -> Result<(), SavedDeckError> {
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

    pub fn get_deck(&self, name: &str) -> Result<[i32; 5], SavedDeckError> {
        let deck = self.decks.get(name).unwrap();
        Ok(deck.cards.clone())
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
    cards: [i32; 5],
}
