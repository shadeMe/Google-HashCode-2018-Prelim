use std::fs::File;
use std::io::{BufReader, BufRead, Write, BufWriter};
use std;
use std::cmp::Ordering;

fn manhattan_dist(a: &Coord, b: &Coord) -> i32 {
	i32::abs(a.x - b.x) + i32::abs(a.y - b.y)
}

pub fn cmp_i32(a: i32, b: i32) -> Ordering {
	if a < b {
		Ordering::Less
	} else if a > b {
		Ordering::Greater
	} else {
		Ordering::Equal
	}
}

#[derive(Copy, Clone, Eq, Hash, Debug)]
pub struct Coord {
	pub x:	i32,
	pub y:	i32
}

impl Coord {
	pub fn new(x: i32, y: i32) -> Coord {
		Coord{x, y}
	}

	pub fn dist(&self, a: &Coord) -> i32 {
		manhattan_dist(self, a)
	}
	
	pub fn is_origin(&self) -> bool {
		self == &Coord::default()
	}
}

impl Default for Coord {
	fn default() -> Self {
		Coord{x: 0, y: 0}
	}
}

impl PartialEq for Coord {
	fn eq(&self, other: &Coord) -> bool {
		self.x == other.x && self.y == other.y
	}
}

#[derive(Copy, Clone)]
pub struct Counter {
	counter:	i32,
	start:		i32,
}

impl Counter {
	pub fn start_zero() -> Counter {
		Counter{counter: 0, start: 0}
	} 	
	
	pub fn start_one() -> Counter {
		Counter{counter: 1, start: 1}
	}
	
	pub fn cur(&self) -> i32 {
		self.counter
	}
	
	pub fn inc(&mut self) -> () {
		self.counter += 1
	}

	pub fn is_start(&self) -> bool {
		self.counter == self.start
	}
}


pub type TimeStep = i32;

#[derive(Debug)]
pub enum FileIOError {
	CouldntOpenFile(std::io::Error),
	LineReadError(std::io::Error),
	LineWriteError(std::io::Error)
}

pub struct FileReader {
	reader:			Box<BufReader<File>>
}

impl FileReader {
	pub fn new(path: &str) -> Result<FileReader, FileIOError> {
		let out = FileReader {
			reader: Box::new(BufReader::new(try!(File::open(path)
															.map_err(|e| FileIOError::CouldntOpenFile(e)))))
		};

		Ok(out)
	}

	pub fn read_all_lines(self) -> Result<Vec<String>, Vec<FileIOError>> {
		let mut lines = Vec::<String>::new();
		let mut errs = Vec::<FileIOError>::new();

		self.reader.lines().for_each(|line|  {
			match line {
				Ok(l) => lines.push(l.to_string()),
				Err(e) => errs.push(FileIOError::LineReadError(e))
			}
		});

		if !errs.is_empty() {
			Err(errs)
		} else {
			Ok(lines)
		}
	}
}

pub struct FileWriter {
	writer:     Box<BufWriter<File>>
}

impl FileWriter {
	pub fn new(path: &str) -> Result<FileWriter, FileIOError> {
		let out = FileWriter {
			writer: Box::new(BufWriter::new(try!(File::create(path)
				.map_err(|e| FileIOError::CouldntOpenFile(e)))))
		};

		Ok(out)
	}

	pub fn write_line(&mut self, line: &str) -> Result<(), FileIOError> {
		self.writer.write_all(line.as_bytes()).map_err(|e| FileIOError::LineWriteError(e))
	}
}

impl Drop for FileWriter {
	fn drop(&mut self) {
		self.writer.flush();
	}
}