use anyhow::{bail, ensure, Result};
use csv::ReaderBuilder;
use derive_new::new;
use id3::{
    frame::{Chapter, TableOfContents},
    Frame, Tag, TagLike, Version,
};
use metadata::MediaFileMetadata;
use model::AuditionCvsRecords;
use std::{fs::copy, path::PathBuf};

mod model;

type Chapters = Vec<Chapter>;

#[derive(Debug)]
pub enum AppliersErrors {
    AudioFileNotCompatible(String),
    ChaptersFileNotCompatible,
    CopyFileError,
}

impl std::fmt::Display for AppliersErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AudioFileNotCompatible(data) => write!(f, "Invalid audio file format {}", data),
            Self::ChaptersFileNotCompatible => write!(f, "Invalid chapter file format"),
            Self::CopyFileError => write!(f, "Error while copying file"),
        }
    }
}

pub fn apply(audition_cvs: PathBuf, mp3_file: PathBuf) -> Result<PathBuf> {
    Applier::new(audition_cvs, mp3_file).apply()
}

#[derive(new)]
struct Applier {
    audition_cvs: PathBuf,
    mp3_file: PathBuf,
}

impl Applier {
    fn apply(&self) -> Result<PathBuf> {
        let cvs = self.load_cvs()?;
        let duration = self.verify_mp3_file()?;
        let tag = Self::build_tag(cvs, duration);
        let new_mp3_file = self.copy_file()?;
        tag.write_to_path(new_mp3_file.clone(), Version::Id3v24)?;
        Ok(new_mp3_file)
    }

    fn copy_file(&self) -> Result<PathBuf> {
        let file_name = self.mp3_file.file_stem().and_then(|file| file.to_str());
        ensure!(file_name.is_some(), AppliersErrors::CopyFileError);

        let new_mp3_file = self
            .mp3_file
            .with_file_name(file_name.unwrap().to_owned() + "_enriched.mp3");
        copy(&self.mp3_file, &new_mp3_file)?;

        Ok(new_mp3_file)
    }

    fn convert_time(time: &str) -> u32 {
        let mut values = [0u32; 4];
        let multipliers = [60 * 60 * 1000u32, 60 * 1000, 1000, 1];

        let (hh_mm_ss, milliseconds) = time.split_once('.').unwrap();
        let mut hh_mm_ss = hh_mm_ss.split(':');

        if hh_mm_ss.clone().count() == 2 {
            values[1] = hh_mm_ss.next().unwrap().parse().unwrap();
            values[2] = hh_mm_ss.next().unwrap().parse().unwrap();
        } else {
            values[0] = hh_mm_ss.next().unwrap().parse().unwrap();
            values[1] = hh_mm_ss.next().unwrap().parse().unwrap();
            values[2] = hh_mm_ss.next().unwrap().parse().unwrap();
        }
        values[3] = milliseconds.parse().unwrap();

        values
            .into_iter()
            .zip(multipliers)
            .map(|(value, multiplier)| value * multiplier)
            .sum()
    }

    fn convert_end_time(id: usize, duration: f64, records: &AuditionCvsRecords) -> u32 {
        if id < records.len() - 1 {
            Self::convert_time(&records[id + 1].start)
        } else {
            duration as u32
        }
    }

    fn build_tag(cvs: AuditionCvsRecords, duration: f64) -> Tag {
        let mut tag = Tag::new();
        let mut chapter_ids = Vec::new();

        Self::build_chapters(cvs, duration)
            .into_iter()
            .for_each(|chapter| {
                chapter_ids.push(chapter.element_id.clone());
                tag.add_frame(chapter);
            });
        tag.add_frame(TableOfContents {
            element_id: "toc".to_string(),
            top_level: true,
            ordered: true,
            elements: chapter_ids,
            frames: vec![Frame::text("TIT2", "chapters-chapz".to_string()); 1],
        });

        tag
    }

    fn build_chapters(records: AuditionCvsRecords, duration: f64) -> Chapters {
        records
            .iter()
            .enumerate()
            .map(|(id, record)| Chapter {
                element_id: id.to_string(),
                start_time: Self::convert_time(&record.start),
                end_time: Self::convert_end_time(id, duration, &records),
                start_offset: 0,
                end_offset: 0,
                frames: vec![Frame::text("TIT2", record.name.clone()); 1],
            })
            .collect()
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

        for record in data.iter() {
            ensure!(
                record.start.contains(':') && record.start.contains('.'),
                AppliersErrors::ChaptersFileNotCompatible
            );
        }
        Ok(data)
    }

    fn verify_mp3_file(&self) -> Result<f64> {
        match MediaFileMetadata::new(&self.mp3_file) {
            Ok(metadata) => match metadata.container_format.as_str() {
                "MP3" => Ok(metadata._duration.unwrap() * 1000f64),
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
    use id3::Tag;

    use crate::{apply, AppliersErrors};

    macro_rules! test_file {
        ($file_name:expr) => {
            concat!(env!("CARGO_MANIFEST_DIR"), "/resources/test/", $file_name)
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
        let new_mp3_file = apply(
            test_file!("valid_chaps.cvs").into(),
            test_file!("audio.mp3").into(),
        );
        assert!(new_mp3_file.is_ok());

        let tag = Tag::read_from_path(new_mp3_file.unwrap());
        assert!(tag.is_ok());
        let tag = tag.unwrap();

        let chapters: Vec<_> = tag.chapters().collect();
        assert!(!chapters.is_empty());
        println!("{:?}", chapters);

        let ctocs: Vec<_> = tag.tables_of_contents().collect();
        assert_eq!(ctocs.len(), 1);
        println!("{:?}", ctocs);

        chapters
            .iter()
            .zip(ctocs.last().unwrap().elements.iter())
            .for_each(|(chap, chap_id)| assert_eq!(chap.element_id, *chap_id));
    }
}
