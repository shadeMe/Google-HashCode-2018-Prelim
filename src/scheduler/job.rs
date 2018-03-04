use util::{Coord, TimeStep};
use std::rc::Rc;


pub type JobId = i32;

pub trait HasJob {
	fn id(&self) -> JobId;
	fn start(&self) -> Coord;
	fn end(&self) -> Coord;
	fn earliest_start(&self) -> TimeStep;
	fn latest_finish(&self) -> TimeStep;
}

struct Job {
	id:				JobId,
	start:			Coord,
	end:			Coord,
	earliest_start:	TimeStep,
	latest_end:		TimeStep,	
}

impl HasJob for Job {
	fn id(&self) -> JobId {
		self.id
	} 
	fn start(&self) -> Coord {
		self.start.clone()
	}
	fn end(&self) -> Coord {
		self.end.clone()
	}
	fn earliest_start(&self) -> TimeStep {
		self.earliest_start	
	}
	fn latest_finish(&self) -> TimeStep {
		self.latest_end
	}
}

pub type JobPtr = Rc<HasJob>;