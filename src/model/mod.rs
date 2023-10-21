use serde::Deserialize;

pub type AuditionCvsRecords = Vec<AuditionCvsRecord>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuditionCvsRecord {
    name: String,
    start: String,
    duration: String,
}
