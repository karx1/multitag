pub mod data;

use data::*;
use id3::Tag as Id3InternalTag;
use id3::TagLike;
use metaflac::Tag as FlacInternalTag;
use mp4ameta::Data as Mp4Data;
use mp4ameta::Fourcc as Mp4Fourcc;
use mp4ameta::Ident as Mp4Ident;
use mp4ameta::Img as Mp4Picture;
use mp4ameta::Tag as Mp4InternalTag;
use std::path::Path;
use std::str::FromStr;
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
    #[error("{0}")]
    FlacError(#[from] metaflac::Error),
    #[error("{0}")]
    Mp4Error(#[from] mp4ameta::Error),
    #[error("Unable to parse timestamp from string")]
    TimestampParseError,
    #[error("given cover image data is not of valid type (bmp, jpeg, png)")]
    InvalidImageFormat,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Tag {
    Id3Tag { inner: Id3InternalTag },
    VorbisFlacTag { inner: FlacInternalTag },
    Mp4Tag { inner: Mp4InternalTag },
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
                let inner = Id3InternalTag::read_from_path(path).unwrap_or_default();
                Ok(Self::Id3Tag { inner })
            }
            "flac" => {
                let inner = FlacInternalTag::read_from_path(path).unwrap_or_default();
                Ok(Self::VorbisFlacTag { inner })
            }
            "mp4" | "m4a" | "m4p" | "m4b" | "m4r" | "m4v" => {
                let inner = Mp4InternalTag::read_from_path(path)?;
                Ok(Self::Mp4Tag { inner })
            }
            _ => Err(Error::UnsupportedFormat),
        }
    }

    pub fn write_to_path<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        match self {
            Self::Id3Tag { inner } => inner.write_to_path(path, id3::Version::Id3v24)?,
            Self::VorbisFlacTag { inner } => inner.write_to_path(path)?,
            Self::Mp4Tag { inner } => inner.write_to_path(path)?,
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
            Self::VorbisFlacTag { inner } => {
                let cover = inner
                    .pictures()
                    .find(|&pic| {
                        matches!(pic.picture_type, metaflac::block::PictureType::CoverFront)
                    })
                    .map(|pic| Picture::from(pic.clone()));

                Some(Album {
                    title: inner.get_vorbis("ALBUM")?.next()?.into(),
                    artist: inner.get_vorbis("ALBUM_ARTIST")?.next()?.into(),
                    cover,
                })
            }
            Self::Mp4Tag { inner } => {
                let cover = inner.artwork().map(Picture::from);
                Some(Album {
                    title: inner.title()?.into(),
                    artist: inner.artist()?.into(),
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
            Self::VorbisFlacTag { inner } => {
                inner.set_vorbis("ALBUM", vec![album.title]);
                inner.set_vorbis("ALBUMARTIST", vec![&album.artist]);
                inner.set_vorbis("ALBUM ARTIST", vec![&album.artist]);
                inner.set_vorbis("ALBUM_ARTIST", vec![&album.artist]);

                if let Some(picture) = album.cover {
                    inner.remove_picture_type(metaflac::block::PictureType::CoverFront);
                    inner.add_picture(
                        picture.mime_type,
                        metaflac::block::PictureType::CoverFront,
                        picture.data,
                    );
                }
            }
            Self::Mp4Tag { inner } => {
                inner.set_album(album.title);
                inner.set_album_artist(album.artist);

                if let Some(picture) = album.cover {
                    let pic: Result<Mp4Picture<Vec<u8>>> = picture.into();
                    match pic {
                        Ok(p) => inner.set_artwork(p),
                        Err(e) => eprintln!("{e}"),
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn title(&self) -> Option<&str> {
        match self {
            Self::Id3Tag { inner } => inner.title(),
            Self::VorbisFlacTag { inner } => inner.get_vorbis("TITLE")?.next(),
            Self::Mp4Tag { inner } => inner.title(),
        }
    }

    pub fn set_title(&mut self, title: &str) {
        match self {
            Self::Id3Tag { inner } => inner.set_title(title),
            Self::VorbisFlacTag { inner } => inner.set_vorbis("TITLE", vec![title]),
            Self::Mp4Tag { inner } => inner.set_title(title),
        }
    }

    #[must_use]
    pub fn artist(&self) -> Option<String> {
        match self {
            Self::Id3Tag { inner } => inner.artist().map(std::string::ToString::to_string),
            Self::VorbisFlacTag { inner } => Some(
                inner
                    .get_vorbis("ARTIST")?
                    .collect::<Vec<&str>>()
                    .join("; "),
            )
            .filter(|s| !s.is_empty()),
            Self::Mp4Tag { inner } => inner.artist().map(std::string::ToString::to_string),
        }
    }

    pub fn set_artist(&mut self, artist: &str) {
        match self {
            Self::Id3Tag { inner } => inner.set_artist(artist),
            Self::VorbisFlacTag { inner } => inner.set_vorbis("ARTIST", vec![artist]),
            Self::Mp4Tag { inner } => inner.set_artist(artist),
        }
    }

    #[must_use]
    pub fn date(&self) -> Option<Timestamp> {
        match self {
            Self::Id3Tag { inner } => inner.date_released().map(std::convert::Into::into),
            Self::VorbisFlacTag { inner } => inner
                .get_vorbis("DATE")?
                .next()
                .map(|s| Timestamp::from_str(s).ok())?,
            Self::Mp4Tag { inner } => inner
                .data()
                .find(|data| matches!(data.0.fourcc().unwrap_or_default().0, [169, 100, 97, 121]))
                .map(|data| -> Option<Timestamp> {
                    Timestamp::from_str(data.1.clone().into_string()?.as_str()).ok()
                })?,
        }
    }

    pub fn set_date(&mut self, timestamp: Timestamp) {
        match self {
            Self::Id3Tag { inner } => inner.set_date_released(timestamp.into()),
            Self::VorbisFlacTag { inner } => inner.set_vorbis(
                "DATE",
                vec![format!(
                    "{:04}-{:02}-{:02}",
                    timestamp.year,
                    timestamp.month.unwrap_or_default(),
                    timestamp.day.unwrap_or_default()
                )],
            ),
            Self::Mp4Tag { inner } => inner.set_data(
                Mp4Fourcc([169, 100, 97, 121]),
                Mp4Data::Utf8(format!(
                    "{:04}-{:02}-{:02}",
                    timestamp.year,
                    timestamp.month.unwrap_or_default(),
                    timestamp.day.unwrap_or_default()
                )),
            ),
        }
    }
}
