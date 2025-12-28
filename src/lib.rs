#![doc = include_str!("../README.md")]

pub mod data;

use data::{Album, Picture, Timestamp};
use id3::Tag as Id3InternalTag;
use id3::TagLike;
use metaflac::Tag as FlacInternalTag;
use mp4ameta::Tag as Mp4InternalTag;
use oggmeta::Tag as OggInternalTag;
use opusmeta::Tag as OpusInternalTag;
use std::convert::Into;
use std::fs::{File, OpenOptions};
use std::io::Cursor;
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::str::FromStr;
use std::string::ToString;
use thiserror::Error;

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
    /// Wrapper around a [`oggmeta::Error`]. See there for more info.
    #[error("{0}")]
    OggError(#[from] oggmeta::Error),
    /// Unable to parse a [`Timestamp`] from a string.
    #[error("Unable to parse timestamp from string")]
    TimestampParseError,
    /// Specified cover image is not of a valid mime type.
    /// Supported types are: bmp, jpg, png.
    #[error("Given cover image data is not of valid type (bmp, jpeg, png)")]
    InvalidImageFormat,
    /// An unspecified I/O error occurred.
    #[error("An I/O error occurred. Please see the contained io::Error for more info.")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// An object containing tags of one of the supported formats.
pub enum Tag {
    Id3Tag { inner: Id3InternalTag },
    VorbisFlacTag { inner: FlacInternalTag },
    Mp4Tag { inner: Mp4InternalTag },
    OpusTag { inner: OpusInternalTag },
    OggTag { inner: OggInternalTag },
}

// reading/writing Tag objects
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

        let file = OpenOptions::new().read(true).open(path)?;
        Tag::read_from(extension, file)
    }

    /// Attempts to read a set of tags from the given reader.
    /// The extension is necessary to determine which backend to use to decode the tags.
    /// `extension` must be one of `[mp3, wav, aiff, flac, mp4, m4a, m4p, m4b, m4r, m4v, opus]`
    ///
    /// # Errors
    /// This function can error if the given extension is not supported by this crate.
    ///
    /// Lastly, an error will be raised if the file type is supported but the reading the tags fails for some
    /// reason other than missing tags.
    /// This could be, for example, that the given reader ended too early or that the tags were
    /// encoded improperly. Please inspect the debug output of the error for more information.
    pub fn read_from<R: Read + Seek>(extension: &str, mut f_in: R) -> Result<Self> {
        match extension {
            "mp3" | "wav" | "aiff" => {
                let res = Id3InternalTag::read_from2(f_in);
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
                let inner = FlacInternalTag::read_from(&mut f_in)?;
                Ok(Self::VorbisFlacTag { inner })
            }
            "mp4" | "m4a" | "m4p" | "m4b" | "m4r" | "m4v" => {
                let res = Mp4InternalTag::read_from(&mut f_in);
                if res
                    .as_ref()
                    .is_err_and(|e: &mp4ameta::Error| matches!(e.kind, mp4ameta::ErrorKind::NoFtyp))
                {
                    return Ok(Self::Mp4Tag {
                        inner: Mp4InternalTag::default(),
                    });
                }
                Ok(Self::Mp4Tag { inner: res? })
            }
            "opus" => {
                let inner = OpusInternalTag::read_from(f_in)?;
                Ok(Self::OpusTag { inner })
            }
            "ogg" => {
                let inner = OggInternalTag::read_from(&mut f_in)?;
                Ok(Self::OggTag { inner })
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
            Self::OggTag { inner } => inner.write_to_path(&path)?,
        }
        Ok(())
    }

    /// Write to a file. The file should already contain valid data of the correct type (e.g. the
    /// file should already contain an opus stream in order to correctly write opus tags).
    ///
    /// The file's cursor should be at the beginning of the file, and it should be opened with
    /// read and write modes set (See [`OpenOptions`] for more info).
    ///
    /// # Errors
    /// This method can error if writing the tags fails, or if accessing the file fails (for
    /// example, if the modes are set wrong).
    pub fn write_to_file(&mut self, file: &mut File) -> Result<()> {
        match self {
            Self::Id3Tag { inner } => inner.write_to_file(file, id3::Version::Id3v24)?,
            Self::VorbisFlacTag { inner } => {
                // this is needed because metaflac doesn't provide a clean way to write without a
                // path
                // see https://github.com/jameshurst/rust-metaflac/issues/19 for more info
                let mut data: Vec<u8> = Vec::new();
                let mut cursor = Cursor::new(&mut data);

                // read the existing tags from the file. Really this is just a way to move the
                // reader to the point directly after the tags and the start of the audio, so we
                // can copy the audio to the cursor after writing our modified tags.
                let _ = FlacInternalTag::read_from(file)?;

                inner.write_to(&mut cursor)?; // write our tags
                std::io::copy(file, &mut cursor)?; // copy the rest of the file to the cursor

                file.rewind()?; // rewind to the beginning of the file
                file.write_all(&data)?; // dump the contents of the vec to the file
            }
            Self::Mp4Tag { inner } => inner.write_to(file)?,
            Self::OpusTag { inner } => inner.write_to(file)?,
            Self::OggTag { inner } => inner.write_to(file)?,
        }

        Ok(())
    }

    /// Write to a byte vector. The vector should already contain valid data of the correct type (e.g. the
    /// vector should already contain an opus stream in order to correctly write opus tags).
    ///
    /// # Errors
    /// This method can error if one of the internal write methods fails. If that happens, the
    /// inner error will contain more information.
    pub fn write_to_vec(&mut self, vec: &mut Vec<u8>) -> Result<()> {
        // we have to clone the vec because id3 and mp4ameta don't implement their traits for
        // Cursor<&mut Vec<u8>>, only Cursor<Vec<u8>>
        let cloned = vec.clone();
        let mut cursor = Cursor::new(cloned);

        match self {
            Self::Id3Tag { inner } => inner.write_to_file(&mut cursor, id3::Version::Id3v24)?,
            Self::VorbisFlacTag { inner } => {
                // See write_to_file method above for rationale
                let mut data: Vec<u8> = Vec::new();
                let mut other_cursor = Cursor::new(&mut data);

                let _ = FlacInternalTag::read_from(&mut cursor)?;

                inner.write_to(&mut other_cursor)?; // write our tags
                std::io::copy(&mut cursor, &mut other_cursor)?; // copy the rest of the data

                cursor.rewind()?; // rewind to the beginning of the cursor
                cursor.write_all(&data)?;
            }
            Self::Mp4Tag { inner } => inner.write_to(&mut cursor)?,
            Self::OpusTag { inner } => inner.write_to(&mut cursor)?,
            Self::OggTag { inner } => inner.write_to(&mut cursor)?,
        }

        *vec = cursor.into_inner();
        Ok(())
    }
}

// creating new tags
impl Tag {
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

    /// Creates an empty set of tags in the Opus format.
    #[must_use]
    pub fn new_empty_opus() -> Self {
        Self::OpusTag {
            inner: OpusInternalTag::default(),
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

                let artist = inner
                    .get_one(&"ALBUM_ARTIST".into())
                    .or_else(|| inner.get_one(&"ALBUMARTIST".into()))
                    .map(Into::into);

                Some(Album {
                    title: inner.get_one(&"ALBUM".into()).map(Into::into),
                    artist,
                    cover,
                })
            }
            Self::OggTag { inner } => {
                let cover = inner
                    .pictures
                    .iter()
                    .find(|pic| matches!(pic.picture_type, oggmeta::PictureType::FrontCover))
                    .map(|pic| Picture::from(pic.clone()));

                Some(Album {
                    title: inner
                        .comments
                        .get("album")?
                        .first()
                        .map(std::convert::Into::into),
                    artist: inner
                        .comments
                        .get("album_artist")?
                        .first()
                        .map(std::convert::Into::into),
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
            Self::OggTag { inner } => {
                if let Some(title) = album.title {
                    inner.comments.insert("album".into(), vec![title]);
                }
                if let Some(album_artist) = album.artist {
                    inner
                        .comments
                        .insert("album_artist".into(), vec![album_artist]);
                }
                if let Some(picture) = album.cover {
                    // Try to decode the image to obtain width/height and color depth
                    inner.pictures.push(picture.data.as_slice().try_into()?);
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
                inner.remove_entries(&"ALBUM".into());
                inner.remove_entries(&"ALBUMARTIST".into());
                inner.remove_entries(&"ALBUM_ARTIST".into());

                let _ = inner.remove_picture_type(opusmeta::picture::PictureType::CoverFront);
            }
            Self::OggTag { inner } => {
                inner.comments.remove("ALBUM");
                inner.comments.remove("ALBUM_ARTIST");
                inner.comments.remove("ALBUMARTIST");
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
            Self::OpusTag { inner } => inner.get_one(&"TITLE".into()).map(String::as_str),
            Self::OggTag { inner } => inner
                .comments
                .get("TITLE")
                .and_then(|o| o.first())
                .map(String::as_str),
        }
    }

    /// Sets the title.
    pub fn set_title(&mut self, title: &str) {
        match self {
            Self::Id3Tag { inner } => inner.set_title(title),
            Self::VorbisFlacTag { inner } => inner.set_vorbis("TITLE", vec![title]),
            Self::Mp4Tag { inner } => inner.set_title(title),
            Self::OpusTag { inner } => inner.add_one("TITLE".into(), title.into()),
            Self::OggTag { inner } => inner
                .comments
                .entry("TITLE".into())
                .or_default()
                .push(title.into()),
        }
    }

    /// Removes any title fields from the file.
    pub fn remove_title(&mut self) {
        match self {
            Self::Id3Tag { inner } => inner.remove_title(),
            Self::VorbisFlacTag { inner } => inner.remove_vorbis("TITLE"),
            Self::Mp4Tag { inner } => inner.remove_title(),
            Self::OpusTag { inner } => {
                inner.remove_entries(&"TITLE".into());
            }
            Self::OggTag { inner } => {
                inner.comments.remove("TITLE");
            }
        }
    }

    /// Gets the artist (note: NOT the album artist!)
    ///
    /// If multiple artist tags are present, they will be joined with a `; `.
    /// Conversely, if no artist tags exist, returns None.
    #[must_use]
    pub fn artist(&self) -> Option<String> {
        // maybe at some point in the future we just want to return the first
        // artist in the Vec?
        self.artists().map(|v| v.join("; "))
    }

    /// Gets all artists (note: NOT the album artist!)
    ///
    /// If no artist tags exist, returns None.
    #[must_use]
    pub fn artists(&self) -> Option<Vec<String>> {
        match self {
            Self::Id3Tag { inner } => inner
                .artists()
                .map(|v: Vec<&str>| v.iter().map(ToString::to_string).collect()),
            Self::VorbisFlacTag { inner } => Some(
                inner
                    .get_vorbis("ARTIST")?
                    .map(ToString::to_string)
                    .collect(),
            ),
            Self::Mp4Tag { inner } => Some(inner.artists().map(ToString::to_string).collect()),
            Self::OpusTag { inner } => inner.get(&"ARTIST".into()).cloned(),
            Self::OggTag { inner } => inner.comments.get("ARTIST").cloned(),
        }
    }

    /// Sets the artist (note: NOT the album artist!)
    pub fn set_artist(&mut self, artist: &str) {
        match self {
            Self::Id3Tag { inner } => inner.set_artist(artist),
            Self::VorbisFlacTag { inner } => inner.set_vorbis("ARTIST", vec![artist]),
            Self::Mp4Tag { inner } => inner.set_artist(artist),
            Self::OpusTag { inner } => {
                inner.remove_entries(&"ARTIST".into());
                inner.add_one("ARTIST".into(), artist.into());
            }
            Self::OggTag { inner } => {
                inner.comments.remove("ARTIST");
                inner.comments.insert("ARTIST".into(), vec![artist.into()]);
            }
        }
    }

    /// Sets multiple artists (node: NOT the album artists!)
    ///
    /// Any existing artists will be removed and the new artists inserted in
    /// their place.
    pub fn set_artists(&mut self, artists: Vec<String>) {
        match self {
            Self::Id3Tag { inner } => inner.set_text_values("TPE1", artists),
            Self::VorbisFlacTag { inner } => {
                inner
                    .vorbis_comments_mut()
                    .comments
                    .insert("ARTIST".into(), artists);
            }
            Self::Mp4Tag { inner } => inner.set_artists(artists),
            Self::OpusTag { inner } => {
                inner.set_entries("ARTIST".into(), artists);
            }
            Self::OggTag { inner } => {
                inner.comments.insert("ARTIST".into(), artists);
            }
        }
    }

    /// Adds one artist to the list of artists. (node: NOT the album artists!)
    ///
    /// Existing artists will remain in the set of tags.
    pub fn add_artist(&mut self, artist: &str) {
        match self {
            Self::Id3Tag { inner } => {
                // this is a lot of allocations for something that should be
                // pretty simple
                // but idk how to avoid this bc of borrow checker
                let mut prev: Vec<String> = inner
                    .artists()
                    .map(|v| v.into_iter().map(ToString::to_string).collect())
                    .unwrap_or_default();

                prev.push(artist.to_owned());
                inner.set_text_values("TPE1", prev);
            }
            Self::VorbisFlacTag { inner } => inner
                .vorbis_comments_mut()
                .comments
                .entry("ARTIST".into())
                .or_default()
                .push(artist.into()),
            Self::Mp4Tag { inner } => inner.add_artist(artist),
            Self::OpusTag { inner } => inner.add_one("ARTIST".into(), artist.into()),
            Self::OggTag { inner } => inner
                .comments
                .entry("ARTIST".into())
                .or_default()
                .push(artist.into()),
        }
    }

    /// Remove all artists (note: NOT the album artists!)
    pub fn remove_artist(&mut self) {
        match self {
            Self::Id3Tag { inner } => inner.remove_artist(),
            Self::VorbisFlacTag { inner } => inner.remove_vorbis("ARTIST"),
            Self::Mp4Tag { inner } => inner.remove_artists(),
            Self::OpusTag { inner } => {
                inner.remove_entries(&"ARTIST".into());
            }
            Self::OggTag { inner } => {
                inner.comments.remove("ARTIST");
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
            Self::Mp4Tag { inner } => inner.year().and_then(|s| Timestamp::from_str(s).ok()),
            Self::OpusTag { inner } => inner
                .get_one(&"DATE".into())
                .and_then(|s| Timestamp::from_str(s).ok()),
            Self::OggTag { inner } => inner
                .comments
                .get("DATE")
                .and_then(|v| Timestamp::from_str(v.first()?).ok()),
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
            Self::Mp4Tag { inner } => inner.set_year(format!(
                "{:04}-{:02}-{:02}",
                timestamp.year,
                timestamp.month.unwrap_or_default(),
                timestamp.day.unwrap_or_default()
            )),
            Self::OpusTag { inner } => {
                inner.remove_entries(&"DATE".into());
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
            Self::OggTag { inner } => {
                inner.comments.remove("DATE");
                inner.comments.insert(
                    "DATE".into(),
                    vec![format!(
                        "{:04}-{:02}-{:02}",
                        timestamp.year,
                        timestamp.month.unwrap_or_default(),
                        timestamp.day.unwrap_or_default()
                    )],
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
            Self::Mp4Tag { inner } => inner.remove_year(),
            Self::OpusTag { inner } => {
                inner.remove_entries(&"DATE".into());
            }
            Self::OggTag { inner } => {
                inner.comments.remove("DATE");
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

    /// Gets lyrics
    /// Since Opus metadata doesn't specify a field for lyrics. It will try to get LYRICS tag field
    #[must_use]
    pub fn lyrics(&self) -> Option<String> {
        match self {
            Self::Id3Tag { inner } => Some(inner.lyrics().map(|l| l.text.clone()).collect()),
            Self::VorbisFlacTag { inner } => Some(inner.get_vorbis("LYRICS")?.collect()),
            Self::Mp4Tag { inner } => Some(inner.userdata.lyrics()?.to_owned()),
            Self::OpusTag { inner } => Some(inner.get_one(&"LYRICS".into())?.to_string()),
            Self::OggTag { inner } => Some(inner.comments.get("LYRICS")?.first()?.to_string()),
        }
    }

    /// Sets lyrics
    pub fn set_lyrics(&mut self, lyrics: &str) {
        match self {
            Self::Id3Tag { inner } => {
                inner.add_frame(id3::frame::Lyrics {
                    lang: String::new(),
                    description: String::new(),
                    text: lyrics.to_string(),
                });
            }
            Self::VorbisFlacTag { inner } => inner.set_vorbis("LYRICS", vec![lyrics]),
            Self::Mp4Tag { inner } => inner.set_lyrics(lyrics),
            Self::OpusTag { inner } => {
                inner.remove_entries(&"LYRICS".into());
                inner.add_one("LYRICS".into(), lyrics.into());
            }
            Self::OggTag { inner } => {
                inner.comments.remove("LYRICS");
                inner.comments.insert("LYRICS".into(), vec![lyrics.into()]);
            }
        }
    }

    /// Removes lyrics
    pub fn remove_lyrics(&mut self) {
        match self {
            Self::Id3Tag { inner } => inner.remove_all_lyrics(),
            Self::VorbisFlacTag { inner } => inner.remove_vorbis("LYRICS"),
            Self::Mp4Tag { inner } => inner.remove_lyrics(),
            Self::OpusTag { inner } => {
                inner.remove_entries(&"LYRICS".into());
            }
            Self::OggTag { inner } => {
                inner.comments.remove("LYRICS");
            }
        }
    }
}
