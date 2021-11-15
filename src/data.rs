use csv::{Reader, ReaderBuilder};
use directories::ProjectDirs;
use inquire::Text;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::game::{Card, Rules, Suit};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(thiserror::Error, Debug)]
pub enum LoadDataError {
    #[error("could not read/write from disk")]
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

    #[error("network request failed")]
    NetworkError(#[from] reqwest::Error),

    #[error("download of {} failed with HTTP {}", 0, 1)]
    DownloadFailed(String, u16),
}

fn append_path<P: AsRef<Path>>(p: &Path, fname: P) -> PathBuf {
    let mut result = p.to_path_buf();
    result.push(fname);
    result
}

const REQUIRED_PATHS: [&str; 5] = [
    "TripleTriadCard.csv",
    "TripleTriadCardResident.csv",
    "TripleTriad.csv",
    "ENpcBase.csv",
    "ENpcResident.csv",
];

pub struct Data {
    pub cards_by_name: HashMap<String, Card>,
    pub card_names: HashMap<i32, String>,
    pub npcs_by_name: HashMap<String, Npc>,
}
impl Data {
    pub fn new(project_dirs: &ProjectDirs) -> Result<Self, LoadDataError> {
        let cache_path = project_dirs.cache_dir();
        let required_paths = REQUIRED_PATHS.map(|fname| append_path(cache_path, fname));
        if required_paths.iter().all(|p| p.exists()) {
            println!("Loading all card and NPC data...");
            let start = Instant::now();
            let result = load_all_data(cache_path)?;
            println!("Loaded data in {:?}", Instant::now() - start);
            Ok(result)
        } else {
            std::fs::create_dir_all(cache_path)?;

            // Download the data from a user-provided URL
            println!("This is the first time the solver has run on this computer, and it needs to download Triple Triad card and NPC data.");
            let repo = Text::new("Please enter the github repository to download from:")
                .prompt()
                .unwrap();

            let repo_parts = repo.split("/").collect::<Vec<_>>();
            let base_url = format!(
                "https://raw.githubusercontent.com/{}/{}/master/csv/",
                repo_parts[0], repo_parts[1]
            );

            println!("Downloading...");
            let client = reqwest::blocking::Client::new();
            let start = Instant::now();
            let results: Vec<usize> = REQUIRED_PATHS
                .map(|fname| (fname, client.clone(), append_path(cache_path, fname)))
                .par_iter()
                .map(|(fname, client, destination)| {
                    let mut url = base_url.clone();
                    url.push_str(fname);

                    let response = client.get(&url).send()?;
                    if !response.status().is_success() {
                        Err(LoadDataError::DownloadFailed(url, response.status().into()))
                    } else {
                        let text = response.text()?;
                        let mut file = File::create(destination)?;
                        file.write_all(text.as_bytes())?;

                        Ok(text.len())
                    }
                })
                .collect::<Result<_, LoadDataError>>()?;

            let duration = Instant::now() - start;
            let total_bytes: usize = results.iter().sum();
            let kib_per_ms = (total_bytes as f64 / 1024f64) / (duration.as_millis() as f64);
            println!(
                "Downloaded card and NPC data in {:?} ({:.2} KiB/sec)",
                duration,
                kib_per_ms * 1000f64
            );

            println!("Loading all card and NPC data...");
            let start = Instant::now();
            let result = load_all_data(cache_path)?;
            println!("Loaded data in {:?}", Instant::now() - start);
            Ok(result)
        }
    }

    pub fn get_card(&self, id: i32) -> Option<&Card> {
        self.card_names
            .get(&id)
            .and_then(|name| self.cards_by_name.get(name))
    }
}

#[derive(Debug)]
pub struct Npc {
    pub fixed_cards: [i32; 5],
    pub variable_cards: [i32; 5],
    pub rules: Rules,
}

pub fn load_all_data<P: AsRef<Path>>(base_path: P) -> Result<Data, LoadDataError> {
    let (name_to_id, card_names) = {
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
    for (name, id) in name_to_id {
        cards_by_name.insert(
            name,
            cards_by_id
                .get(&id)
                .cloned()
                .ok_or(LoadDataError::MissingCardData(id))?,
        );
    }

    if cards_by_name.len() != cards_by_id.len() {
        return Err(LoadDataError::MissingNames);
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
        card_names,
        npcs_by_name,
    })
}

fn load_npc_names<P: AsRef<Path>>(
    path: P,
    ids: HashSet<i32>,
) -> Result<HashMap<i32, String>, LoadDataError> {
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
) -> Result<HashMap<i32, i32>, LoadDataError> {
    let mut csv = open_csv(path)?;

    let mut result = HashMap::new();
    for record in csv.records().skip(2) {
        let record = record?;

        let top_id = record[0].parse()?;
        for i in 0..32 {
            let id = record[i + 3].parse()?;

            if npc_ids.contains_key(&id) {
                result.insert(id, top_id);
                break;
            }
        }
    }

    Ok(result)
}

fn load_tt_npc_data<P: AsRef<Path>>(path: P) -> Result<HashMap<i32, Npc>, LoadDataError> {
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

fn load_card_names<P: AsRef<Path>>(
    path: P,
) -> Result<(HashMap<String, i32>, HashMap<i32, String>), LoadDataError> {
    let mut csv = open_csv(path)?;

    let mut name_to_id = HashMap::new();
    let mut id_to_name = HashMap::new();
    // Skip the first row since it's just type information, and the second row is the dummy card
    for record in csv.records().skip(2) {
        let record = record?;
        let id = record[0].parse()?;
        let name = record[1].to_string();

        name_to_id.insert(name.clone(), id);
        id_to_name.insert(id, name);
    }

    Ok((name_to_id, id_to_name))
}

fn load_cards_resident<P: AsRef<Path>>(path: P) -> Result<HashMap<i32, Card>, LoadDataError> {
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
            _ => return Err(LoadDataError::UnknownSuit(record[7].to_string())),
        };

        result.insert(id, Card::new(n, s, w, e, suit));
    }

    Ok(result)
}

fn open_csv<P: AsRef<Path>>(path: P) -> Result<Reader<BufReader<File>>, LoadDataError> {
    let mut file = BufReader::new(File::open(path)?);

    // throw away the first line
    let mut buf = String::new();
    file.read_line(&mut buf)?;

    Ok(ReaderBuilder::new().has_headers(true).from_reader(file))
}
