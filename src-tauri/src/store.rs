use std::sync::{Arc, Mutex};
use crate::database::Database;
use crate::services::pricing::PricingService;
use crate::services::incremental_import::IncrementalImporter;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub pricing: Arc<PricingService>,
    pub importer: Mutex<Option<IncrementalImporter>>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        let pricing = Arc::new(PricingService::new());
        Self {
            db: Arc::new(Mutex::new(db)),
            pricing,
            importer: Mutex::new(None),
        }
    }

    pub fn set_importer(&self, imp: IncrementalImporter) {
        let mut slot = self.importer.lock().expect("importer mutex poisoned");
        *slot = Some(imp);
    }
}