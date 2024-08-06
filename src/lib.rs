mod data;

use data::*;
use id3::Tag as Id3InternalTag;
use id3::TagLike;
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
    pub fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .ok_or(Error::NoFileExtension)?
            .to_str()
            .ok_or(Error::InvalidFileExtension)?;
        match extension {
            "mp3" | "wav" | "aiff" => {
                let inner = Id3InternalTag::read_from_path(path)?;
                Ok(Self::Id3Tag { inner })
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

    #[must_use]
    pub fn get_album_info(&self) -> Option<Album> {
        match self {
            Self::Id3Tag { inner } => {
                let cover = inner
                    .pictures()
                    .find(|&pic| matches!(pic.picture_type, id3::frame::PictureType::CoverFront))
                    .map(|pic| Picture::from(pic.clone()));

                Some(Album {
                    title: inner.album()?.into(),
                    artist: inner.album_artist()?.into(),
                    cover,
                })
            }
        }
    }

    pub fn set_album_info(&mut self, album: Album) {
        match self {
            Self::Id3Tag { inner } => {
                inner.set_album(album.title);
                inner.set_album_artist(album.artist);

                if let Some(pic) = album.cover {
                    inner.add_frame(id3::frame::Picture {
                        mime_type: pic.mime_type,
                        picture_type: id3::frame::PictureType::CoverFront,
                        description: String::new(),
                        data: pic.data,
                    });
                }
            }
        }
    }
}
