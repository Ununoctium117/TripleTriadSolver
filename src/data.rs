use csv::{Reader, ReaderBuilder};

use crate::game::{Card, Rules, Suit};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

#[derive(thiserror::Error, Debug)]
pub enum LoadError {
    #[error("could not read from disk")]
    IoError(#[from] std::io::Error),

    #[error("invalid CSV")]
    CsvError(#[from] csv::Error),

    #[error("found card with unknown suit {0}")]
    UnknownSuit(String),

    #[error("couldn't parse integer in CSV")]
    IntParse(#[from] std::num::ParseIntError),

    #[error("no data for card with ID {0}")]
    MissingCardData(i32),

    #[error("missing name data for card(s)")]
    MissingNames,
}

pub struct Data {
    pub cards_by_name: HashMap<String, Card>,
    pub cards_by_id: HashMap<i32, Card>,
    pub npcs_by_name: HashMap<String, Npc>,
}

#[derive(Debug)]
pub struct Npc {
    pub fixed_cards: [i32; 5],
    pub variable_cards: [i32; 5],
    pub rules: Rules,
}

pub fn load_all_data<P: AsRef<Path>>(base_path: P) -> Result<Data, LoadError> {
    let card_names = {
        let mut card_names_path = base_path.as_ref().to_path_buf();
        card_names_path.push("TripleTriadCard.csv");
        load_card_names(card_names_path)?
    };

    let cards_by_id = {
        let mut resident_path = base_path.as_ref().to_path_buf();
        resident_path.push("TripleTriadCardResident.csv");
        load_cards_resident(resident_path)?
    };

    let mut cards_by_name = HashMap::new();
    for (name, id) in card_names {
        cards_by_name.insert(
            name,
            cards_by_id
                .get(&id)
                .cloned()
                .ok_or(LoadError::MissingCardData(id))?,
        );
    }

    if cards_by_name.len() != cards_by_id.len() {
        return Err(LoadError::MissingNames);
    }

    let npcs_by_id = {
        let mut npc_path = base_path.as_ref().to_path_buf();
        npc_path.push("TripleTriad.csv");
        load_tt_npc_data(npc_path)?
    };

    let npc_id_map = {
        let mut npc_path = base_path.as_ref().to_path_buf();
        npc_path.push("ENpcBase.csv");
        load_npc_id_map(npc_path, &npcs_by_id)?
    };

    let mut npc_names = {
        let mut path = base_path.as_ref().to_path_buf();
        path.push("ENpcResident.csv");
        load_npc_names(path, npc_id_map.values().copied().collect())?
    };

    let mut npcs_by_name = HashMap::new();
    for (id, npc) in npcs_by_id {
        if let Some(mapped_id) = npc_id_map.get(&id) {
            if let Some(name) = npc_names.remove(mapped_id) {
                npcs_by_name.insert(name, npc);
            } else {
                println!("Missing name for NPC {} (mapped: {})", id, mapped_id);
            }
        } else {
            println!("Missing ID mapping for NPC {}", id);
        }
    }

    Ok(Data {
        cards_by_name,
        cards_by_id,
        npcs_by_name,
    })
}

fn load_npc_names<P: AsRef<Path>>(
    path: P,
    ids: HashSet<i32>,
) -> Result<HashMap<i32, String>, LoadError> {
    let mut csv = open_csv(path)?;

    let mut result = HashMap::new();
    for record in csv.records().skip(2) {
        let record = record?;

        if record[1].is_empty() {
            continue;
        }

        let id = record[0].parse()?;
        if ids.contains(&id) {
            result.insert(id, record[1].to_string());
        }
    }

    Ok(result)
}

fn load_npc_id_map<P: AsRef<Path>>(
    path: P,
    npc_ids: &HashMap<i32, Npc>,
) -> Result<HashMap<i32, i32>, LoadError> {
    let mut csv = open_csv(path)?;

    let mut result = HashMap::new();
    for record in csv.records().skip(2) {
        let record = record?;

        let top_id = record[0].parse()?;
        for i in 0..32 {
            let id = record[i + 3].parse()?;
            if id == 0 {
                break;
            }

            if npc_ids.contains_key(&id) {
                result.insert(id, top_id);
                break;
            }
        }
    }

    Ok(result)
}

fn load_tt_npc_data<P: AsRef<Path>>(path: P) -> Result<HashMap<i32, Npc>, LoadError> {
    let mut csv = open_csv(path)?;

    let mut result = HashMap::new();
    for record in csv.records().skip(2) {
        let record = record?;

        let id = record[0].parse()?;

        let mut fixed_cards = [0; 5];
        for i in 0..5 {
            fixed_cards[i] = record[i + 1].parse()?;
        }

        let mut variable_cards = [0; 5];
        for i in 0..5 {
            variable_cards[i] = record[i + 6].parse()?;
        }

        let rule0 = record[11].parse()?;
        let rule1 = record[12].parse()?;
        let mut rules = Rules::default();
        rules.add_rule_from_csv(rule0);
        rules.add_rule_from_csv(rule1);

        result.insert(
            id,
            Npc {
                fixed_cards,
                variable_cards,
                rules,
            },
        );
    }

    Ok(result)
}

fn load_card_names<P: AsRef<Path>>(path: P) -> Result<HashMap<String, i32>, LoadError> {
    let mut csv = open_csv(path)?;

    let mut result = HashMap::new();
    // Skip the first row since it's just type information, and the second row is the dummy card
    for record in csv.records().skip(2) {
        let record = record?;
        result.insert(record[1].to_string(), record[0].parse()?);
    }

    Ok(result)
}

fn load_cards_resident<P: AsRef<Path>>(path: P) -> Result<HashMap<i32, Card>, LoadError> {
    let mut csv = open_csv(path)?;

    let mut result = HashMap::new();
    // Skip the first row since it's just type information, and the second row is the dummy card
    for record in csv.records().skip(2) {
        let record = record?;

        let id = record[0].parse()?;
        let n = record[2].parse()?;
        let s = record[3].parse()?;
        let w = record[4].parse()?;
        let e = record[5].parse()?;
        let suit = match &record[7] {
            "0" => None,
            "1" => Some(Suit::Primal),
            "2" => Some(Suit::Scion),
            "3" => Some(Suit::Beastman),
            "4" => Some(Suit::Garlean),
            _ => return Err(LoadError::UnknownSuit(record[7].to_string())),
        };

        result.insert(id, Card::new(n, s, w, e, suit));
    }

    Ok(result)
}

fn open_csv<P: AsRef<Path>>(path: P) -> Result<Reader<BufReader<File>>, LoadError> {
    let mut file = BufReader::new(File::open(path)?);

    // throw away the first line
    let mut buf = String::new();
    file.read_line(&mut buf)?;

    Ok(ReaderBuilder::new().has_headers(true).from_reader(file))
}
