extern crate docopt;
extern crate rustc_serialize;

use docopt::Docopt;
use std::fs::{File, Metadata, read_dir, symlink_metadata};
use std::io::{Read, Result, Write};
use std::path::Path;

const MAX_PREFIX_SIZE: usize = 4 * 1024;
type CounterArray = [[u32; 256]; MAX_PREFIX_SIZE];
type FileReadBuffer = [u8; MAX_PREFIX_SIZE];

fn process_regular_file<P: AsRef<Path>>(
    path: P,
    counters: &mut CounterArray,
    buffer: &mut FileReadBuffer,
    total_counter: &mut u32
) -> Result<()> {
    let mut file = try!(File::open(path));
    let mut read_total = 0usize;
    while read_total < MAX_PREFIX_SIZE {
        let (_, unfilled) = buffer.split_at_mut(read_total);
        let read_len = try!(file.read(unfilled));
        if read_len == 0 { break; }
        read_total += read_len;
    }
    for (ix, byte) in buffer.split_at(read_total).0.iter().enumerate() {
        counters[ix][*byte as usize] += 1;
    }
    *total_counter += 1;
    if (*total_counter % 1000) == 0 {
        println!("processed {} files", total_counter);
    }
    return Ok(());
}

fn process_directory<P: AsRef<Path>>(
    path: P,
    counters: &mut CounterArray,
    buffer: &mut FileReadBuffer,
    total_counter: &mut u32
) -> Result<()> {
    for entry in try!(read_dir(path)) {
        let entry = try!(entry);
        let entry_path = entry.path();
        let entry_meta = try!(entry.metadata());
        try!(process_entry(entry_path, entry_meta, counters, buffer, total_counter));
    }
    return Ok(());
}

fn process_entry<P: AsRef<Path>>(
    path: P,
    metadata: Metadata,
    counters: &mut CounterArray,
    buffer: &mut FileReadBuffer,
    total_counter: &mut u32
) -> Result<()> {
    if metadata.is_file() {
        return process_regular_file(path, counters, buffer, total_counter);
    } else if metadata.is_dir() {
        return process_directory(path, counters, buffer, total_counter);
    } else {
        return Ok(());
    }
}

fn process_root(
    root: &String,
    counters: &mut CounterArray,
    buffer: &mut FileReadBuffer,
    total_counter: &mut u32
) -> Result<()> {
    println!("processing root: {}", root);
    let root = Path::new(root);
    let meta = try!(symlink_metadata(root));
    return process_entry(root, meta, counters, buffer, total_counter);
}

const USAGE: &'static str = "
Usage:
    file-analyzer <output> <dir>...
";

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_output: String,
    arg_dir: Vec<String>
}

fn main() {
    let args: Args = Docopt::new(USAGE).and_then(|d| d.decode()).unwrap_or_else(|e| e.exit());
    let mut counters = [[0u32; 256]; MAX_PREFIX_SIZE];
    let mut buffer = [0u8; MAX_PREFIX_SIZE];
    let mut total_counter = 0u32;
    for root_dir in args.arg_dir {
        process_root(&root_dir, &mut counters, &mut buffer, &mut total_counter).unwrap();
    }
    println!("finished {} files, writing result file...", total_counter);
    let mut out_file = File::create(Path::new(&args.arg_output)).unwrap();
    for row in counters.iter() {
        for (ix, byte_count) in row.iter().enumerate() {
            if ix != 0 {
                out_file.write_all(b",").unwrap();
            }
            write!(out_file, "{}", byte_count).unwrap();
        }
        write!(out_file, "\n").unwrap();
    }
}
