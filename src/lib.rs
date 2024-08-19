//! `multitag` is a crate for reading and writing audio metadata of various formats
//!
//! We currently support reading and writing metadata to mp3, wav, aiff, flac, and mp4/m4a/...
//! files, with support for more formats on the way.

pub mod data;

use data::*;
use id3::Tag as Id3InternalTag;
use id3::TagLike;
use metaflac::Tag as FlacInternalTag;
use mp4ameta::Data as Mp4Data;
use mp4ameta::Fourcc as Mp4Fourcc;
use mp4ameta::Ident as Mp4Ident;
use mp4ameta::Tag as Mp4InternalTag;
use opusmeta::Tag as OpusInternalTag;
use std::convert::Into;
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;

const DATE_FOURCC: Mp4Fourcc = Mp4Fourcc([169, 100, 97, 121]);

/// Error type.
///
/// Describes various errors that this crate could produce.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// A file does not have a file extension.
    #[error("Given file does not have a file extension")]
    NoFileExtension,
    /// The file *extension* does not contain valid unicode
    #[error("File extension must be valid unicode")]
    InvalidFileExtension,
    /// The format of the specified audio file is not currently supported by this crate.
    #[error("Unsupported audio format")]
    UnsupportedAudioFormat,
    /// Wrapper around an [`id3::Error`]. See there for more info.
    #[error("{0}")]
    Id3Error(#[from] id3::Error),
    /// Wrapper around a [`metaflac::Error`]. See there for more info.
    #[error("{0}")]
    FlacError(#[from] metaflac::Error),
    /// Wrapper around a [`mp4ameta::Error`]. See there for more info.
    #[error("{0}")]
    Mp4Error(#[from] mp4ameta::Error),
    /// Wrapper around a [`opusmeta::Error`]. See there for more info.
    #[error("{0}")]
    OpusError(#[from] opusmeta::Error),
    /// Unable to parse a [`Timestamp`] from a string.
    #[error("Unable to parse timestamp from string")]
    TimestampParseError,
    /// Specified cover image is not of a valid mime type.
    /// Supported types are: bmp, jpg, png.
    #[error("Given cover image data is not of valid type (bmp, jpeg, png)")]
    InvalidImageFormat,
}

pub type Result<T> = std::result::Result<T, Error>;

/// An object containing tags of one of the supported formats.
pub enum Tag {
    Id3Tag { inner: Id3InternalTag },
    VorbisFlacTag { inner: FlacInternalTag },
    Mp4Tag { inner: Mp4InternalTag },
    OpusTag { inner: OpusInternalTag },
}

impl Tag {
    /// Attempts to read a set of tags from the given path.
    ///
    /// # Errors
    /// This function could error if the given path has a file extension which contains invalid
    /// unicode or if the given path does not have a file extension at all.
    ///
    /// This function could also error if the given path has a valid extension but the extension is
    /// not among the types supported by this crate.
    ///
    /// Lastly, an error will be raised if the file type is supported but the reading the tags fails for some
    /// reason other than missing tags.
    pub fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .ok_or(Error::NoFileExtension)?
            .to_str()
            .ok_or(Error::InvalidFileExtension)?;
        match extension {
            "mp3" | "wav" | "aiff" => {
                let res = Id3InternalTag::read_from_path(path);
                if res
                    .as_ref()
                    .is_err_and(|e: &id3::Error| matches!(e.kind, id3::ErrorKind::NoTag))
                {
                    return Ok(Self::Id3Tag {
                        inner: Id3InternalTag::default(),
                    });
                }
                Ok(Self::Id3Tag { inner: res? })
            }
            "flac" => {
                let inner = FlacInternalTag::read_from_path(path)?;
                Ok(Self::VorbisFlacTag { inner })
            }
            "mp4" | "m4a" | "m4p" | "m4b" | "m4r" | "m4v" => {
                let res = Mp4InternalTag::read_from_path(path);
                if res
                    .as_ref()
                    .is_err_and(|e: &mp4ameta::Error| matches!(e.kind, mp4ameta::ErrorKind::NoTag))
                {
                    return Ok(Self::Mp4Tag {
                        inner: Mp4InternalTag::default(),
                    });
                }
                Ok(Self::Mp4Tag { inner: res? })
            }
            "opus" => {
                let inner = OpusInternalTag::read_from_path(path)?;
                Ok(Self::OpusTag { inner })
            }
            _ => Err(Error::UnsupportedAudioFormat),
        }
    }

    /// Attempts to write the tags to the indicated path.
    /// # Errors
    /// This function will error if writing the tags fails in any way.
    pub fn write_to_path<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        match self {
            Self::Id3Tag { inner } => inner.write_to_path(path, id3::Version::Id3v24)?,
            Self::VorbisFlacTag { inner } => inner.write_to_path(path)?,
            Self::Mp4Tag { inner } => inner.write_to_path(path)?,
            Self::OpusTag { inner } => inner.write_to_path(path)?,
        };
        Ok(())
    }

    /// Creates an empty set of tags in the ID3 format.
    #[must_use]
    pub fn new_empty_id3() -> Self {
        Self::Id3Tag {
            inner: Id3InternalTag::default(),
        }
    }

    /// Creates an empty set of tags in the FLAC format.
    #[must_use]
    pub fn new_empty_flac() -> Self {
        Self::VorbisFlacTag {
            inner: FlacInternalTag::default(),
        }
    }

    /// Creates an empty set of tags in the MP4 format.
    #[must_use]
    pub fn new_empty_mp4() -> Self {
        Self::Mp4Tag {
            inner: Mp4InternalTag::default(),
        }
    }
}

impl Tag {
    /// Gets the album information. If the `album` or `album_artist` fields are not present in the
    /// audio file, this method returns None.
    #[must_use]
    pub fn get_album_info(&self) -> Option<Album> {
        match self {
            Self::Id3Tag { inner } => {
                let cover = inner
                    .pictures()
                    .find(|&pic| matches!(pic.picture_type, id3::frame::PictureType::CoverFront))
                    .map(|pic| Picture::from(pic.clone()));

                Some(Album {
                    title: inner.album().map(std::convert::Into::into),
                    artist: inner.album_artist().map(std::convert::Into::into),
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
                    title: inner
                        .get_vorbis("ALBUM")
                        .and_then(|mut v| v.next())
                        .map(std::convert::Into::into),
                    artist: inner
                        .get_vorbis("ALBUM_ARTIST")
                        .and_then(|mut v| v.next())
                        .map(std::convert::Into::into),
                    cover,
                })
            }
            Self::Mp4Tag { inner } => {
                let cover = inner.artwork().map(Picture::from);
                Some(Album {
                    title: inner.album().map(std::convert::Into::into),
                    artist: inner.album_artist().map(Into::into),
                    cover,
                })
            }
            Self::OpusTag { inner } => {
                let cover = inner
                    .get_picture_type(opusmeta::picture::PictureType::CoverFront)
                    .map(Picture::from);

                Some(Album {
                    title: inner
                        .get("ALBUM".into())
                        .and_then(|v| v.first())
                        .map(Into::into),
                    artist: inner
                        .get("ALBUM_ARTIST".into())
                        .and_then(|v| v.first())
                        .map(Into::into),
                    cover,
                })
            }
        }
    }

    /// Sets the album information of the audio track.
    /// # Errors
    /// This function will error if `album.cover` has an invalid or unsupported MIME type.
    /// Supported MIME types are: `image/bmp`, `image/jpeg`, `image/png`
    pub fn set_album_info(&mut self, album: Album) -> Result<()> {
        match self {
            Self::Id3Tag { inner } => {
                if let Some(title) = album.title {
                    inner.set_album(title);
                }
                if let Some(album_artist) = album.artist {
                    inner.set_album_artist(album_artist);
                }

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
                if let Some(title) = album.title {
                    inner.set_vorbis("ALBUM", vec![title]);
                }
                if let Some(album_artist) = album.artist {
                    inner.set_vorbis("ALBUMARTIST", vec![&album_artist]);
                    inner.set_vorbis("ALBUM ARTIST", vec![&album_artist]);
                    inner.set_vorbis("ALBUM_ARTIST", vec![&album_artist]);
                }

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
                if let Some(title) = album.title {
                    inner.set_album(title);
                }
                if let Some(album_artist) = album.artist {
                    inner.set_album_artist(album_artist);
                }

                if let Some(picture) = album.cover {
                    inner.set_artwork(picture.try_into()?);
                }
            }
            Self::OpusTag { inner } => {
                if let Some(title) = album.title {
                    inner.add_one("ALBUM".into(), title);
                }
                if let Some(album_artist) = album.artist {
                    inner.add_one("ALBUMARTIST".into(), album_artist.clone());
                    inner.add_one("ALBUM_ARTIST".into(), album_artist);
                }

                let opus_pic = album.cover.map(std::convert::Into::into).map(
                    |mut pic: opusmeta::picture::Picture| {
                        pic.picture_type = opusmeta::picture::PictureType::CoverFront;
                        pic
                    },
                );

                if let Some(pic) = opus_pic {
                    inner.add_picture(&pic)?;
                }
            }
        }
        Ok(())
    }

    /// Removes all album infofrom the audio track.
    pub fn remove_all_album_info(&mut self) {
        match self {
            Self::Id3Tag { inner } => {
                inner.remove_album();
                inner.remove_album_artist();
                inner.remove_picture_by_type(id3::frame::PictureType::CoverFront);
            }
            Self::VorbisFlacTag { inner } => {
                inner.remove_vorbis("ALBUM");
                inner.remove_vorbis("ALBUMARTIST");
                inner.remove_vorbis("ALBUM ARTIST");
                inner.remove_vorbis("ALBUM_ARTIST");

                inner.remove_picture_type(metaflac::block::PictureType::CoverFront);
            }
            Self::Mp4Tag { inner } => {
                inner.remove_album();
                inner.remove_album_artists();
                inner.remove_artworks();
            }
            Self::OpusTag { inner } => {
                inner.remove_entries("ALBUM".into());
                inner.remove_entries("ALBUMARTIST".into());
                inner.remove_entries("ALBUM_ARTIST".into());

                let _ = inner.remove_picture_type(opusmeta::picture::PictureType::CoverFront);
            }
        }
    }

    /// Gets the title.
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        match self {
            Self::Id3Tag { inner } => inner.title(),
            Self::VorbisFlacTag { inner } => inner.get_vorbis("TITLE")?.next(),
            Self::Mp4Tag { inner } => inner.title(),
            Self::OpusTag { inner } => inner.get("TITLE".into())?.first().map(String::as_str),
        }
    }

    /// Sets the title.
    pub fn set_title(&mut self, title: &str) {
        match self {
            Self::Id3Tag { inner } => inner.set_title(title),
            Self::VorbisFlacTag { inner } => inner.set_vorbis("TITLE", vec![title]),
            Self::Mp4Tag { inner } => inner.set_title(title),
            Self::OpusTag { inner } => inner.add_one("TITLE".into(), title.into()),
        }
    }

    /// Removes any title fields from the file.
    pub fn remove_title(&mut self) {
        match self {
            Self::Id3Tag { inner } => inner.remove_title(),
            Self::VorbisFlacTag { inner } => inner.remove_vorbis("TITLE"),
            Self::Mp4Tag { inner } => inner.remove_title(),
            Self::OpusTag { inner } => {
                inner.remove_entries("TITLE".into());
            }
        }
    }

    /// Gets the artist (note: NOT the album artist!)
    /// If multiple ARTIST tags are present, they will be joined with a `; `
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
            Self::OpusTag { inner } => Some(inner.get("ARTIST".into())?.join("; ")),
        }
    }

    /// Sets the artist (note: NOT the album artist!)
    pub fn set_artist(&mut self, artist: &str) {
        match self {
            Self::Id3Tag { inner } => inner.set_artist(artist),
            Self::VorbisFlacTag { inner } => inner.set_vorbis("ARTIST", vec![artist]),
            Self::Mp4Tag { inner } => inner.set_artist(artist),
            Self::OpusTag { inner } => {
                inner.remove_entries("ARTIST".into());
                inner.add_one("ARTIST".into(), artist.into());
            }
        }
    }

    /// Removes the artist (note: NOT the album artist!)
    pub fn remove_artist(&mut self) {
        match self {
            Self::Id3Tag { inner } => inner.remove_artist(),
            Self::VorbisFlacTag { inner } => inner.remove_vorbis("ARTIST"),
            Self::Mp4Tag { inner } => inner.remove_artists(),
            Self::OpusTag { inner } => {
                inner.remove_entries("ARTIST".into());
            }
        }
    }

    /// Gets the date
    /// # Format-specific
    /// In id3, this method corresponds to the `date_released` field.
    #[must_use]
    pub fn date(&self) -> Option<Timestamp> {
        match self {
            Self::Id3Tag { inner } => inner.date_released().map(std::convert::Into::into),
            Self::VorbisFlacTag { inner } => inner
                .get_vorbis("DATE")?
                .next()
                .and_then(|s| Timestamp::from_str(s).ok()),
            Self::Mp4Tag { inner } => inner
                .data()
                .find(|data| matches!(data.0.fourcc().unwrap_or_default(), DATE_FOURCC))
                .map(|data| -> Option<Timestamp> {
                    Timestamp::from_str(data.1.clone().into_string()?.as_str()).ok()
                })?,
            Self::OpusTag { inner } => inner
                .get("DATE".into())?
                .first()
                .and_then(|s| Timestamp::from_str(s).ok()),
        }
    }

    /// Sets the date
    /// # Format-specific
    /// In id3, this method corresponds to the `date_released` field.
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
                DATE_FOURCC,
                Mp4Data::Utf8(format!(
                    "{:04}-{:02}-{:02}",
                    timestamp.year,
                    timestamp.month.unwrap_or_default(),
                    timestamp.day.unwrap_or_default()
                )),
            ),
            Self::OpusTag { inner } => {
                inner.remove_entries("DATE".into());
                inner.add_one(
                    "DATE".into(),
                    format!(
                        "{:04}-{:02}-{:02}",
                        timestamp.year,
                        timestamp.month.unwrap_or_default(),
                        timestamp.day.unwrap_or_default()
                    ),
                );
            }
        }
    }

    /// Removes the date
    /// # Format-specific
    /// In id3, this method corresponds to the `date_released` field.
    pub fn remove_date(&mut self) {
        match self {
            Self::Id3Tag { inner } => inner.remove_date_released(),
            Self::VorbisFlacTag { inner } => inner.remove_vorbis("DATE"),
            Self::Mp4Tag { inner } => inner.remove_data_of(&DATE_FOURCC),
            Self::OpusTag { inner } => {
                inner.remove_entries("DATE".into());
            }
        }
    }

    /// Copies the information of this [`Tag`] to another. The target [`Tag`] can be any of the
    /// supported formats.
    pub fn copy_to(&self, other: &mut Self) {
        if let Some(album) = self.get_album_info() {
            // This should be ok since if the tag was read then the mime type should already be valid
            let _ = other.set_album_info(album);
        }

        if let Some(title) = self.title() {
            other.set_title(title);
        }

        if let Some(artist) = self.artist() {
            other.set_artist(&artist);
        }

        if let Some(date) = self.date() {
            other.set_date(date);
        }
    }
}
