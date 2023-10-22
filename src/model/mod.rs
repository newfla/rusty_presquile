use serde::Deserialize;

pub type AuditionCvsRecords = Vec<AuditionCvsRecord>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuditionCvsRecord {
    pub name: String,
    pub start: String,
}
