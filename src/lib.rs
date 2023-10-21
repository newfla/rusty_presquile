use anyhow::{bail, ensure, Result};
use csv::ReaderBuilder;
use derive_new::new;
use metadata::MediaFileMetadata;
use model::AuditionCvsRecords;
use std::path::PathBuf;

mod model;

#[derive(Debug)]
pub enum AppliersErrors {
    AudioFileNotCompatible(String),
    ChaptersFileNotCompatible,
}

impl std::fmt::Display for AppliersErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AudioFileNotCompatible(data) => write!(f, "Invalid audio file format {}", data),
            AppliersErrors::ChaptersFileNotCompatible => write!(f, "Invalid chapter file format"),
        }
    }
}

pub fn apply(audition_cvs: PathBuf, mp3_file: PathBuf) -> Result<()> {
    Applier::new(audition_cvs, mp3_file).apply()
}

#[derive(new)]
struct Applier {
    audition_cvs: PathBuf,
    mp3_file: PathBuf,
}

impl Applier {
    fn apply(&self) -> Result<()> {
        let duration = self.verify_mp3_file()?;
        let cvs = self.load_cvs()?;
        Ok(())
    }

    fn load_cvs(&self) -> Result<AuditionCvsRecords> {
        let mut rdr = ReaderBuilder::new()
            .delimiter(b'\t')
            .trim(csv::Trim::All)
            .from_path(self.audition_cvs.as_path())?;

        let (error, data): (Vec<_>, Vec<_>) = rdr.deserialize().partition(|line| line.is_err());
        let data: AuditionCvsRecords = data.into_iter().map(|f| f.unwrap()).collect();
        ensure!(
            error.is_empty() && !data.is_empty(),
            AppliersErrors::ChaptersFileNotCompatible
        );
        Ok(data)
    }

    fn verify_mp3_file(&self) -> Result<f64> {
        match MediaFileMetadata::new(&self.mp3_file) {
            Ok(metadata) => match metadata.container_format.as_str() {
                "MP3" => Ok(metadata._duration.unwrap()),
                _ => bail!(AppliersErrors::AudioFileNotCompatible(
                    metadata.container_format
                )),
            },
            Err(_) => bail!(AppliersErrors::AudioFileNotCompatible(
                self.mp3_file.display().to_string()
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{apply, AppliersErrors};

    macro_rules! test_file {
        ($fname:expr) => {
            concat!(env!("CARGO_MANIFEST_DIR"), "/resources/test/", $fname)
        };
    }

    #[test]
    fn test_not_audio() {
        assert!(apply(
            test_file!("valid_chaps.cvs").into(),
            test_file!("file.txt").into(),
        )
        .is_err_and(|e| match e.downcast_ref() {
            Some(AppliersErrors::AudioFileNotCompatible(_)) => true,
            _ => false,
        }))
    }

    #[test]
    fn test_not_mp3_audio() {
        assert!(apply(
            test_file!("valid_chaps.cvs").into(),
            test_file!("audio.ogg").into(),
        )
        .is_err_and(|e| match e.downcast_ref() {
            Some(AppliersErrors::AudioFileNotCompatible(_)) => true,
            _ => false,
        }))
    }

    #[test]
    fn test_not_cvs() {
        assert!(apply(
            test_file!("file.txt").into(),
            test_file!("audio.mp3").into(),
        )
        .is_err_and(|e| match e.downcast_ref() {
            Some(AppliersErrors::ChaptersFileNotCompatible) => true,
            _ => false,
        }))
    }

    #[test]
    fn test_invalid_cvs() {
        assert!(apply(
            test_file!("invalid_chaps.cvs").into(),
            test_file!("audio.mp3").into(),
        )
        .is_err_and(|e| match e.downcast_ref() {
            Some(AppliersErrors::ChaptersFileNotCompatible) => true,
            _ => false,
        }))
    }

    #[test]
    fn test_best_case() {
        assert!(apply(
            test_file!("valid_chaps.cvs").into(),
            test_file!("audio.mp3").into()
        )
        .is_ok());
    }
}
