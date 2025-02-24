use anyhow::{Result, bail, ensure};
use csv::ReaderBuilder;
use derive_new::new;
use id3::{
    Frame, Tag, TagLike, Version,
    frame::{Chapter, TableOfContents},
};
use metadata::MediaFileMetadata;
use model::AuditionCvsRecords;
use std::{fs::copy, iter, path::PathBuf, thread};
use thiserror::Error;

mod model;

pub enum Mode {
    Sequential,
    Parallel,
}

#[derive(Debug, Error)]
pub enum AppliersErrors {
    #[error("Invalid audio file format {0}")]
    AudioFileNotCompatible(String),
    #[error("Invalid chapter file format")]
    ChaptersFileNotCompatible,
    #[error("Error while copying file")]
    CopyFile,
    #[error("Thread has been interrupted")]
    ThreadInterrupted,
}

pub fn apply(audition_cvs: PathBuf, mp3_file: PathBuf, parallel: Mode) -> Result<PathBuf> {
    match parallel {
        Mode::Sequential => Applier::new(audition_cvs, mp3_file).apply_seq(),
        Mode::Parallel => Applier::new(audition_cvs, mp3_file).apply_parallel(),
    }
}

#[derive(new)]
struct Applier {
    audition_cvs: PathBuf,
    mp3_file: PathBuf,
}

impl Applier {
    fn apply_seq(&self) -> Result<PathBuf> {
        let cvs = self.load_cvs()?;
        let duration = self.verify_mp3_file()?;
        let tag = Self::build_tag(cvs, duration);
        let new_mp3_file = self.copy_file()?;
        tag.write_to_path(new_mp3_file.clone(), Version::Id3v24)?;
        Ok(new_mp3_file)
    }

    fn apply_parallel(&self) -> Result<PathBuf> {
        use crate::AppliersErrors::ThreadInterrupted;

        let (tag, new_mp3_file) = thread::scope(|s| {
            let cvs = s.spawn(|| self.load_cvs());
            let duration = s.spawn(|| self.verify_mp3_file());
            let new_mp3_file = s.spawn(|| self.copy_file());

            let cvs = cvs.join().map_err(|_| ThreadInterrupted)??;
            let duration = duration.join().map_err(|_| ThreadInterrupted)??;

            let tag = Self::build_tag(cvs, duration);
            let new_mp3_file = new_mp3_file.join().map_err(|_| ThreadInterrupted)??;
            anyhow::Ok((tag, new_mp3_file))
        })?;

        tag.write_to_path(&new_mp3_file, Version::Id3v24)?;
        Ok(new_mp3_file)
    }

    fn copy_file(&self) -> Result<PathBuf> {
        let file_name = self.mp3_file.file_stem().and_then(|file| file.to_str());
        ensure!(file_name.is_some(), AppliersErrors::CopyFile);

        let new_mp3_file = self
            .mp3_file
            .with_file_name(file_name.unwrap().to_owned() + "_enriched.mp3");
        copy(&self.mp3_file, &new_mp3_file)?;

        Ok(new_mp3_file)
    }

    fn convert_time(time: &str) -> u32 {
        //Precalculate 100*(pow(60,n)) to avoid inconsistency between bench runs
        let multipliers = [1000, 60 * 1000, 60 * 60 * 1000u32];

        let (hh_mm_ss, milliseconds) = time.split_once('.').unwrap();
        let hh_mm_ss = hh_mm_ss.split(':');

        hh_mm_ss
            .map(|v| v.parse::<u32>().unwrap())
            .rev()
            .enumerate()
            .map(|(idx, val)| val * multipliers[idx])
            .chain(iter::once(milliseconds.parse().unwrap()))
            .sum()
    }

    fn build_tag(cvs: AuditionCvsRecords, duration: f64) -> Tag {
        let mut tag = Tag::new();
        let chapter_ids: Vec<_> = Self::build_chapters(cvs, duration)
            .map(|chapter| {
                let id = chapter.element_id.clone();
                tag.add_frame(chapter);
                id
            })
            .collect();
        tag.add_frame(TableOfContents {
            element_id: "toc".to_string(),
            top_level: true,
            ordered: true,
            elements: chapter_ids,
            frames: vec![Frame::text("TIT2", "chapters-chapz"); 1],
        });

        tag
    }

    fn build_chapters(records: AuditionCvsRecords, duration: f64) -> impl Iterator<Item = Chapter> {
        let mut end_time = duration as u32;
        records
            .into_iter()
            .enumerate()
            .rev()
            .map(move |(id, record)| {
                let start_time = Self::convert_time(&record.start);
                let ch = Chapter {
                    element_id: id.to_string(),
                    start_time,
                    end_time,
                    start_offset: 0,
                    end_offset: 0,
                    frames: vec![Frame::text("TIT2", record.name); 1],
                };
                end_time = start_time;
                ch
            })
            .rev()
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

    use crate::{AppliersErrors, Mode, apply};

    macro_rules! test_file {
        ($file_name:expr) => {
            concat!(env!("CARGO_MANIFEST_DIR"), "/resources/test/", $file_name)
        };
    }

    #[test]
    fn test_not_audio_parallel() {
        assert!(
            apply(
                test_file!("valid_chaps.cvs").into(),
                test_file!("file.txt").into(),
                Mode::Parallel,
            )
            .is_err_and(|e| match e.downcast_ref() {
                Some(AppliersErrors::AudioFileNotCompatible(_)) => true,
                _ => false,
            })
        )
    }

    #[test]
    fn test_not_mp3_audio_parallel() {
        assert!(
            apply(
                test_file!("valid_chaps.cvs").into(),
                test_file!("audio.ogg").into(),
                Mode::Parallel,
            )
            .is_err_and(|e| match e.downcast_ref() {
                Some(AppliersErrors::AudioFileNotCompatible(_)) => true,
                _ => false,
            })
        )
    }

    #[test]
    fn test_not_cvs_parallel() {
        assert!(
            apply(
                test_file!("file.txt").into(),
                test_file!("audio.mp3").into(),
                Mode::Parallel,
            )
            .is_err_and(|e| match e.downcast_ref() {
                Some(AppliersErrors::ChaptersFileNotCompatible) => true,
                _ => false,
            })
        )
    }

    #[test]
    fn test_invalid_cvs_parallel() {
        assert!(
            apply(
                test_file!("invalid_chaps.cvs").into(),
                test_file!("audio.mp3").into(),
                Mode::Parallel,
            )
            .is_err_and(|e| match e.downcast_ref() {
                Some(AppliersErrors::ChaptersFileNotCompatible) => true,
                _ => false,
            })
        )
    }

    #[test]
    fn test_best_case_parallel() {
        let new_mp3_file = apply(
            test_file!("valid_chaps.cvs").into(),
            test_file!("audio.mp3").into(),
            Mode::Parallel,
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

    #[test]
    fn test_not_audio_seq() {
        assert!(
            apply(
                test_file!("valid_chaps.cvs").into(),
                test_file!("file.txt").into(),
                Mode::Sequential,
            )
            .is_err_and(|e| match e.downcast_ref() {
                Some(AppliersErrors::AudioFileNotCompatible(_)) => true,
                _ => false,
            })
        )
    }

    #[test]
    fn test_not_mp3_audio_seq() {
        assert!(
            apply(
                test_file!("valid_chaps.cvs").into(),
                test_file!("audio.ogg").into(),
                Mode::Sequential,
            )
            .is_err_and(|e| match e.downcast_ref() {
                Some(AppliersErrors::AudioFileNotCompatible(_)) => true,
                _ => false,
            })
        )
    }

    #[test]
    fn test_not_cvs_seq() {
        assert!(
            apply(
                test_file!("file.txt").into(),
                test_file!("audio.mp3").into(),
                Mode::Sequential,
            )
            .is_err_and(|e| match e.downcast_ref() {
                Some(AppliersErrors::ChaptersFileNotCompatible) => true,
                _ => false,
            })
        )
    }

    #[test]
    fn test_invalid_cvs_seq() {
        assert!(
            apply(
                test_file!("invalid_chaps.cvs").into(),
                test_file!("audio.mp3").into(),
                Mode::Sequential,
            )
            .is_err_and(|e| match e.downcast_ref() {
                Some(AppliersErrors::ChaptersFileNotCompatible) => true,
                _ => false,
            })
        )
    }

    #[test]
    fn test_best_case_seq() {
        let new_mp3_file = apply(
            test_file!("valid_chaps.cvs").into(),
            test_file!("audio.mp3").into(),
            Mode::Sequential,
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
