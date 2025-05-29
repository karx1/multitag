use multitag::Tag;
use std::env::args;
use std::fs::File;
use std::io::Cursor;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    let path = PathBuf::from(args().nth(1).unwrap());

    // Option 1: read from path
    let tag = Tag::read_from_path(&path).unwrap();
    println!("{:#?}", tag.title());

    // Option 2: read from reader
    let mut f = File::open(&path).unwrap();

    let mut data = Vec::new();
    f.read_to_end(&mut data).unwrap();

    let cursor = Cursor::new(data);
    // You can also just pass in f instead of creating a cursor since Files are Read + Seek
    let extension = path.extension().unwrap().to_str().unwrap();
    let tag = Tag::read_from(extension, cursor).unwrap();
    println!("{:#?}", tag.title());
}
