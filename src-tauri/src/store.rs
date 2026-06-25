use std::sync::{Arc, Mutex};
use crate::database::Database;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        Self { db: Arc::new(Mutex::new(db)) }
    }
}