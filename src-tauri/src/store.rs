use std::sync::{Arc, Mutex};
use crate::database::Database;
use crate::services::pricing::PricingService;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub pricing: Arc<PricingService>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        let pricing = Arc::new(PricingService::new());
        Self { db: Arc::new(Mutex::new(db)), pricing }
    }
}