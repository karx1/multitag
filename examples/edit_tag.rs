use std::env::args;
use std::fs::OpenOptions;
use std::io::Seek;
use std::path::PathBuf;

use multitag::Tag;

fn main() {
    let path = PathBuf::from(args().nth(1).unwrap());
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        // .truncate(true)
        .open(&path)
        .unwrap();

    let extension = path.extension().unwrap().to_str().unwrap();

    let mut tag = Tag::read_from(extension, &file).unwrap();

    file.rewind().unwrap();

    let title = args().skip(2).collect::<Vec<String>>().join(" ");

    tag.set_title(&title);
    tag.write_to_file(&mut file).unwrap();
}
