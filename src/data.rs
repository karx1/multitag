//! Useful types for representing audio metadata information.
//!
//! The types in this module are typically returned by methods on [`Tag`](crate::Tag).

use crate::{Error, Result};
use id3::frame::Picture as Id3Picture;
use id3::frame::Timestamp as Id3Timestamp;
use metaflac::block::Picture as FlacPicture;
use mp4ameta::Img as Mp4Picture;
use mp4ameta::ImgFmt as Mp4ImageFmt;
use oggmeta::Picture as OggPicture;
use opusmeta::picture::Picture as OpusPicture;
use std::str::FromStr;

/// Represents the album that a song is part of.
#[derive(Clone, Debug, Default)]
pub struct Album {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub cover: Option<Picture>,
}

/// Stores picture data.
#[derive(Clone, Debug)]
pub struct Picture {
    pub data: Vec<u8>,
    pub mime_type: String,
}

impl From<Id3Picture> for Picture {
    fn from(value: Id3Picture) -> Self {
        Self {
            data: value.data,
            mime_type: value.mime_type,
        }
    }
}

impl From<FlacPicture> for Picture {
    fn from(value: FlacPicture) -> Self {
        Self {
            data: value.data,
            mime_type: value.mime_type,
        }
    }
}

impl From<Mp4Picture<&[u8]>> for Picture {
    fn from(value: Mp4Picture<&[u8]>) -> Self {
        Self {
            data: value.data.to_vec(),
            mime_type: match value.fmt {
                Mp4ImageFmt::Bmp => "image/bmp".into(),
                Mp4ImageFmt::Jpeg => "image/jpeg".into(),
                Mp4ImageFmt::Png => "image/png".into(),
            },
        }
    }
}

impl TryFrom<Picture> for Mp4Picture<Vec<u8>> {
    type Error = Error;

    fn try_from(value: Picture) -> Result<Self> {
        let image_fmt = match value.mime_type.as_str() {
            "image/bmp" => Ok(Mp4ImageFmt::Bmp),
            "image/jpeg" => Ok(Mp4ImageFmt::Jpeg),
            "image/png" => Ok(Mp4ImageFmt::Png),
            _ => Err(Error::InvalidImageFormat),
        }?;

        Ok(Self {
            fmt: image_fmt,
            data: value.data,
        })
    }
}

impl From<OpusPicture> for Picture {
    fn from(value: OpusPicture) -> Self {
        Self {
            data: value.data,
            mime_type: value.mime_type,
        }
    }
}

impl From<OggPicture> for Picture {
    fn from(value: OggPicture) -> Self {
        Self {
            data: value.data,
            mime_type: value.media_type,
        }
    }
}

impl From<Picture> for OpusPicture {
    fn from(value: Picture) -> Self {
        let mut picture = OpusPicture::new();
        picture.mime_type = value.mime_type;
        picture.data = value.data;

        picture
    }
}

impl std::fmt::Display for Picture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Picture data ({}, {} bytes)",
            self.mime_type,
            self.data.len()
        )
    }
}

/// Represents a date and time according to the ID3v2.4 spec.
#[derive(Clone, Copy, Debug, Default)]
pub struct Timestamp {
    pub year: i32,
    pub month: Option<u8>,
    pub day: Option<u8>,
    pub hour: Option<u8>,
    pub minute: Option<u8>,
    pub second: Option<u8>,
}

impl From<Id3Timestamp> for Timestamp {
    fn from(value: Id3Timestamp) -> Self {
        Self {
            year: value.year,
            month: value.month,
            day: value.day,
            hour: value.hour,
            minute: value.minute,
            second: value.second,
        }
    }
}

impl From<Timestamp> for Id3Timestamp {
    fn from(value: Timestamp) -> Self {
        Self {
            year: value.year,
            month: value.month,
            day: value.day,
            hour: value.hour,
            minute: value.minute,
            second: value.second,
        }
    }
}

impl FromStr for Timestamp {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(Id3Timestamp::from_str(s)
            .map_err(|_| Error::TimestampParseError)?
            .into())
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Id3Timestamp::from(*self))
    }
}
