fn manhattan_dist(a: &Coord, b: &Coord) -> i32 {
	i32::abs(a.x - b.x) + i32::abs(a.y - b.y)
}

#[derive(Copy, Clone)]
pub struct Coord {
	pub x:	i32,
	pub y:	i32
}

impl Coord {
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
	c:	i32,
}

impl Counter {
	pub fn start_zero() -> Counter {
		Counter{c: 0}
	} 	
	
	pub fn start_one() -> Counter {
		Counter{c: 1}
	}
	
	pub fn cur(&self) -> i32 {
		self.c
	}
	
	pub fn inc(&mut self) -> () {
		self.c += 1
	} 
}


pub type TimeStep = i32;
