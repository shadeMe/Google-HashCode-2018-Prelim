use std::fs::File;
use std::io::{BufReader, BufRead};
use std;

fn manhattan_dist(a: &Coord, b: &Coord) -> i32 {
	i32::abs(a.x - b.x) + i32::abs(a.y - b.y)
}

#[derive(Copy, Clone, Eq, Hash)]
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

pub struct FileReader {
	line_counter:	Counter,
	reader:			BufReader<File>,
}

impl FileReader {
	pub fn new(path: &str) -> Result<FileReader, std::io::Error> {
		let mut out = FileReader{
			line_counter: Counter::start_zero(),
			reader: BufReader::new(try!(File::open(path))),
		};

		Ok(out)
	}

	pub fn read_all_lines<F>(&mut self, delegate: F)
		where F: FnOnce(Result<(&str, &Counter), std::io::Error>)
	{
		self.reader.lines().for_each(|l| {
			delegate(l.map(|s| (s.as_str(), &self.line_counter)));
			self.line_counter.inc();
		});
	}
}