use std::path::PathBuf;
use anyhow::{Result, bail, anyhow};
use derive_new::new;
use metadata::MediaFileMetadata;
pub fn apply (audition_cvs: PathBuf, mp3_file: PathBuf) -> Result<()> {
    Applier::new(audition_cvs, mp3_file).apply()
}

#[derive(new)]
struct Applier {
    audition_cvs: PathBuf,
    mp3_file: PathBuf
}

impl Applier {
    fn apply(&self) -> Result<()> {
        //self.verify_mp3_file()
        Ok(())
    }

    fn load_cvs(&self) -> Result<()> {

        Ok(())
    }

    fn verify_mp3_file(&self) -> Result<f64> {
        let metadata = MediaFileMetadata::new(&self.mp3_file)?;
        match metadata.container_format.as_str() {
            "MPEG"=> Ok(metadata._duration.unwrap()),
            _ => bail!("Invalid container format: {}", metadata.container_format)
        }
    }
}