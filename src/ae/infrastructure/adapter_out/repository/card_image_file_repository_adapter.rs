use crate::application::error::{AppError, InfraError};
use crate::application::repository::CardImageRepository;
use crate::domain::card::CardId;
use crate::domain::card_image::{CardFace, CardImages, card_image_file_name};
use async_trait::async_trait;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Card images stored as files in a folder shared with the frontend, which serves them.
pub struct CardImageFileRepositoryAdapter {
    dir: PathBuf,
}

impl CardImageFileRepositoryAdapter {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Writes to a temporary file in the same folder, then renames it: a reader sees either the
    /// previous file or the new one, never a partial write.
    async fn write_atomically(&self, file_name: &str, bytes: &[u8]) -> Result<(), InfraError> {
        let target = self.dir.join(file_name);
        let temp = self
            .dir
            .join(format!(".{file_name}.{}.tmp", Uuid::new_v4()));
        if let Err(e) = tokio::fs::write(&temp, bytes).await {
            return Err(io_error(&temp, e));
        }
        if let Err(e) = tokio::fs::rename(&temp, &target).await {
            let _ = tokio::fs::remove_file(&temp).await;
            return Err(io_error(&target, e));
        }
        Ok(())
    }

    async fn remove_if_exists(&self, file_name: &str) -> Result<(), InfraError> {
        let path = self.dir.join(file_name);
        match tokio::fs::remove_file(&path).await {
            Err(e) if e.kind() != ErrorKind::NotFound => Err(io_error(&path, e)),
            _ => Ok(()),
        }
    }
}

fn io_error(path: &Path, e: std::io::Error) -> InfraError {
    InfraError::RepositoryError(format!("{}: {e}", path.display()))
}

#[async_trait]
impl CardImageRepository for CardImageFileRepositoryAdapter {
    #[tracing::instrument(name = "card_image_repo.save", skip_all, fields(sentry.op = "file"))]
    async fn save(&self, card_id: &CardId, images: &CardImages) -> Result<(), AppError> {
        tokio::fs::create_dir_all(&self.dir)
            .await
            .map_err(|e| io_error(&self.dir, e))?;

        self.write_atomically(
            &card_image_file_name(card_id, CardFace::Front),
            &images.front,
        )
        .await?;
        let back_name = card_image_file_name(card_id, CardFace::Back);
        match &images.back {
            Some(back) => self.write_atomically(&back_name, back).await?,
            None => self.remove_if_exists(&back_name).await?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::language_code::LanguageCode;

    /// A fresh folder under the system temp dir, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!("ae-card-images-{}", Uuid::new_v4())))
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn card_id() -> CardId {
        CardId::new("ISD", "51", LanguageCode::FR)
    }

    fn files(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    #[tokio::test]
    async fn save_writes_the_front_byte_for_byte_and_creates_the_folder() {
        let dir = TempDir::new();
        let adapter = CardImageFileRepositoryAdapter::new(&dir.0);

        adapter
            .save(
                &card_id(),
                &CardImages {
                    front: b"front".to_vec(),
                    back: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(files(&dir.0), vec!["ISD_51_FR.webp"]);
        assert_eq!(
            std::fs::read(dir.0.join("ISD_51_FR.webp")).unwrap(),
            b"front"
        );
    }

    #[tokio::test]
    async fn save_writes_both_faces_of_a_double_faced_card() {
        let dir = TempDir::new();
        let adapter = CardImageFileRepositoryAdapter::new(&dir.0);

        adapter
            .save(
                &card_id(),
                &CardImages {
                    front: b"front".to_vec(),
                    back: Some(b"back".to_vec()),
                },
            )
            .await
            .unwrap();

        assert_eq!(files(&dir.0), vec!["ISD_51_FR.webp", "ISD_51_FR_back.webp"]);
        assert_eq!(
            std::fs::read(dir.0.join("ISD_51_FR_back.webp")).unwrap(),
            b"back"
        );
    }

    #[tokio::test]
    async fn save_replaces_previous_files_and_removes_a_stale_back() {
        let dir = TempDir::new();
        let adapter = CardImageFileRepositoryAdapter::new(&dir.0);
        adapter
            .save(
                &card_id(),
                &CardImages {
                    front: b"old front".to_vec(),
                    back: Some(b"old back".to_vec()),
                },
            )
            .await
            .unwrap();

        adapter
            .save(
                &card_id(),
                &CardImages {
                    front: b"new front".to_vec(),
                    back: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(files(&dir.0), vec!["ISD_51_FR.webp"]);
        assert_eq!(
            std::fs::read(dir.0.join("ISD_51_FR.webp")).unwrap(),
            b"new front"
        );
    }

    #[tokio::test]
    async fn a_failed_rename_leaves_no_temporary_file() {
        let dir = TempDir::new();
        // A non-empty folder where the image goes: a file cannot be renamed over it.
        std::fs::create_dir_all(dir.0.join("ISD_51_FR.webp").join("blocker")).unwrap();
        let adapter = CardImageFileRepositoryAdapter::new(&dir.0);

        let result = adapter
            .save(
                &card_id(),
                &CardImages {
                    front: b"front".to_vec(),
                    back: None,
                },
            )
            .await;

        assert!(result.is_err());
        assert_eq!(files(&dir.0), vec!["ISD_51_FR.webp"]);
    }

    #[tokio::test]
    async fn save_fails_when_the_folder_cannot_be_created() {
        let dir = TempDir::new();
        std::fs::write(&dir.0, b"a file, not a folder").unwrap();
        let adapter = CardImageFileRepositoryAdapter::new(dir.0.join("images"));

        let result = adapter
            .save(
                &card_id(),
                &CardImages {
                    front: b"front".to_vec(),
                    back: None,
                },
            )
            .await;

        assert!(result.is_err());
        let _ = std::fs::remove_file(&dir.0);
    }
}
