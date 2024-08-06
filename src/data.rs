use id3::frame::Picture as Id3Picture;

#[derive(Clone, Debug)]
pub struct Album {
    pub title: String,
    pub artist: String,
    pub cover: Option<Picture>,
}

#[derive(Clone, Debug)]
pub struct Picture {
    pub data: Vec<u8>,
    pub mime_type: String,
}

impl From<Id3Picture> for Picture {
    fn from(value: Id3Picture) -> Self {
        Picture {
            data: value.data,
            mime_type: value.mime_type,
        }
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
