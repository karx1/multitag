use id3::Tag as Id3InternalTag;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Given file does not have a file extension")]
    NoFileExtension,
    #[error("File extension must be valid unicode")]
    InvalidFileExtension,
    #[error("Unsupported file format")]
    UnsupportedFormat,
    #[error("{0}")]
    Id3Error(#[from] id3::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Tag {
    Id3Tag { inner: Id3InternalTag },
}

impl Tag {
    pub fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Tag> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .ok_or(Error::NoFileExtension)?
            .to_str()
            .ok_or(Error::InvalidFileExtension)?;
        match extension {
            "mp3" | "wav" | "aiff" => {
                let inner = Id3InternalTag::read_from_path(path)?;
                Ok(Tag::Id3Tag { inner })
            }
            _ => Err(Error::UnsupportedFormat),
        }
    }

    pub fn write_to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        match self {
            Self::Id3Tag { inner } => inner.write_to_path(path, id3::Version::Id3v24)?,
        };
        Ok(())
    }
}
