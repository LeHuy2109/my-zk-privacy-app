use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

const NOTES_PATH: &str = ".zkprivacy-notes.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub amount: u64,
    pub secret: String,
    pub commitment: String,
    pub tx_hash: Option<String>,
    pub timestamp: u64,
    pub network: String,
    pub spent: bool,
}

#[derive(Default, Serialize, Deserialize)]
pub struct NoteStore {
    pub notes: Vec<Note>,
}

impl NoteStore {
    pub fn load() -> Result<Self> {
        let path = PathBuf::from(NOTES_PATH);
        if !path.exists() {
            return Ok(Self::default());
        }
        let json = fs::read_to_string(path).context("Failed to read note store")?;
        serde_json::from_str(&json).context("Failed to parse note store")
    }

    pub fn save(&self) -> Result<()> {
        fs::write(NOTES_PATH, serde_json::to_string_pretty(self)?)
            .context("Failed to write note store")
    }

    pub fn add(&mut self, mut note: Note) -> Result<Note> {
        if note.id.is_empty() {
            note.id = next_id();
        }
        self.notes.push(note.clone());
        self.save()?;
        Ok(note)
    }

    pub fn get(&self, id: &str) -> Result<Note> {
        self.notes
            .iter()
            .find(|note| note.id == id)
            .cloned()
            .with_context(|| format!("Note `{id}` not found. Run `zkprivacy notes list`."))
    }

    pub fn import(&mut self, note: Note) -> Result<()> {
        if self.notes.iter().any(|n| n.id == note.id) {
            anyhow::bail!("Note `{}` already exists", note.id);
        }
        self.notes.push(note);
        self.save()
    }
}

pub fn notes_path() -> &'static str {
    NOTES_PATH
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn next_id() -> String {
    format!("note-{}", now())
}
