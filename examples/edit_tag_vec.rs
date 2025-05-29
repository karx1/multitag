use multitag::Tag;
use std::env::args;
use std::fs::OpenOptions;
use std::io::Cursor;
use std::io::Read;
use std::io::Seek;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let path = PathBuf::from(args().nth(1).unwrap());
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        // .truncate(true)
        .open(&path)
        .unwrap();

    let extension = path.extension().unwrap().to_str().unwrap();

    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    let mut cursor = Cursor::new(&mut data);

    let mut tag = Tag::read_from(extension, &mut cursor).unwrap();

    cursor.rewind().unwrap();

    let title = args().skip(2).collect::<Vec<String>>().join(" ");

    tag.set_title(&title);
    tag.write_to_vec(&mut data).unwrap();

    file.rewind().unwrap();
    file.write_all(&data).unwrap();
}
