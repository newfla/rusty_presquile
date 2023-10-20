use anyhow::{bail, Result};
use derive_new::new;
use metadata::MediaFileMetadata;
use std::path::PathBuf;

#[derive(Debug)]
pub enum AppliersErrors {
    AudioFileNotCompatible(String),
}

impl std::fmt::Display for AppliersErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AudioFileNotCompatible(data) => write!(f, "Invalid audio file format {}", data),
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
        self.verify_mp3_file()?;
        self.load_cvs()?;
        Ok(())
    }

    fn load_cvs(&self) -> Result<()> {
        Ok(())
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
            test_file!("valid_chaps.json").into(),
            test_file!("file.txt").into(),
        )
        .is_err_and(|e| match e.downcast_ref() {
            Some(AppliersErrors::AudioFileNotCompatible(_)) => true,
            None => false,
        }))
    }

    #[test]
    fn test_mp3_audio() {
        assert!(apply(
            test_file!("valid_chaps.json").into(),
            test_file!("audio.mp3").into()
        )
        .is_ok());
    }

    #[test]
    fn test_not_mp3_audio() {
        assert!(apply(
            test_file!("valid_chaps.json").into(),
            test_file!("audio.ogg").into(),
        )
        .is_err_and(|e| match e.downcast_ref() {
            Some(AppliersErrors::AudioFileNotCompatible(_)) => true,
            None => false,
        }))
    }
}
