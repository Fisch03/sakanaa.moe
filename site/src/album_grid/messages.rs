use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerMessage {
    Fetch {
        id: String,
    },
    SetSize {
        size: f64,
        primary: String,
        secondary: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MainMessage {
    Ready,
    Cover { id: String, data_url: String },
}
