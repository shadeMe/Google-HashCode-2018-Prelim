use util::{Coord, TimeStep, FileReader, Counter};
use vehicle::Vehicle;
use std::collections::{HashMap, BinaryHeap, HashSet};
use std::rc::Rc;
use std::cmp::Ordering;
use std::io::Error;
use std::hash::Hash;

pub type JobId = i32;

pub trait HasJob: Ord {
	fn id(&self) -> JobId;
	fn start(&self) -> Coord;
	fn end(&self) -> Coord;
	fn earliest_start(&self) -> TimeStep;
	fn latest_finish(&self) -> TimeStep;
}

#[derive(Eq, Hash)]
pub struct Job {
	id:				JobId,
	start:			Coord,
	end:			Coord,
	earliest_start:	TimeStep,
	latest_end:		TimeStep,
}

impl Job {
	fn cmp_ascending(&self, other: &Job) -> Ordering {
		if self.earliest_start < other.earliest_start {
			Ordering::Less
		} else if self.earliest_start > other.earliest_start {
			Ordering::Greater
		} else {
			// TODO compare start and end coords?
			Ordering::Equal
		}
	}
}

/// Sorted by the earliest start first
impl PartialOrd for Job {
	fn partial_cmp(&self, other: &Job) -> Option<Ordering> {
		Some(other.cmp_ascending(&self))
	}
}
impl PartialEq for Job {
	fn eq(&self, other: &Job) -> bool {
		other.cmp_ascending(&self) == Ordering::Equal
	}
}
impl Ord for Job {
	fn cmp(&self, other: &Job) -> Ordering {
		// We flip the ordering as we need a min-heap
		other.cmp_ascending(&self)
	}
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
type VehPtr = Rc<Vehicle>;

/// Context for a single simulation timestep 
pub struct NewTick {
	pub current_step:	TimeStep,
}

/// Context for a new job
pub struct NewJob {
	pub job:			JobPtr,
	pub current_step:	TimeStep,
}

/// Output of a single simulation timestep
pub enum TickComplete {
	/// Continue execution
	Continue,
	/// Update current job
	Reschedule(JobPtr),
	/// Assign new job
	JobComplete(Coord),
}

struct JobScheduler {
	num_rows:			i32,
	num_cols:			i32,
	num_vehicles:		i32,
	num_rides:			i32,
	ride_bonus:			i32,
	max_tsteps:			TimeStep,

	current_step:		TimeStep,
	fleet:              Vec<VehPtr>,
	jobs:               Vec<JobPtr>,

	grid:               HashMap<Coord, HashSet<VehPtr>>,
	rem_jobs:           BinaryHeap<JobPtr>,
}

impl JobScheduler {
	fn new(input: &mut FileReader) -> JobScheduler {
		let mut out = JobScheduler {
			num_rows: 0,
			num_cols: 0,
			num_vehicles: 0,
			num_rides: 0,
			ride_bonus: 0,
			max_tsteps: 0,
			current_step: 0,
			fleet: Vec::default(),
			jobs: Vec::default(),
			grid: HashMap::default(),
			rem_jobs: BinaryHeap::default(),
		};

		let reader = |r : Result<(&str, &Counter), Error>| {
			match r {
				Ok((l, c)) => {
					let mut splits: Vec<i32> = l.split(' ').map(|s| s.parse::<i32>().unwrap()).collect();
					assert_eq!(splits.len(), 6);

					match c.is_start() {
						true => {
							out.num_rows = splits[0];
							out.num_cols = splits[1];
							out.num_vehicles = splits[2];
							out.num_rides = splits[3];
							out.ride_bonus = splits[4];
							out.max_tsteps = splits[5];

							out.current_step = 0;
							out.fleet = Vec::with_capacity(out.num_vehicles as usize);
							out.jobs = Vec::with_capacity(out.num_rides as usize);
							out.grid = HashMap::with_capacity((out.num_rows * out.num_cols) as usize);
							out.rem_jobs = BinaryHeap::with_capacity(out.num_rides as usize);
						},
						_ => {
							let x_start = splits[0];
							let y_start = splits[1];
							let x_end = splits[2];
							let y_end = splits[3];
							let early = splits[4];
							let late = splits[5];

							let jerb = Job{
								id: c.cur() - 1,       // the jobs start from the second line in the input file
								start: Coord::new(x_start, y_start),
								end: Coord::new(x_end, y_end),
								earliest_start: early,
								latest_end: late,
							};

							out.jobs.push(Rc::new(jerb));
						}
					};
				},
				Err(e) => print!("Error reading input. Error: {:?}", e),
			};
		};

		// parse input
		input.read_all_lines(reader);

		// init fleet and grid
		for i in 0..out.num_vehicles {
			out.fleet.push(Rc::new(Vehicle::new(i)));
		}

		for x in 0..out.num_rows {
			for y in 0..out.num_cols {
				let mut set = HashSet::<VehPtr>::new();
				let coord = Coord::new(x, y);

				// all vehicles start at the origin
				if coord.is_origin() {
					for v in out.fleet {
						set.insert(v.clone());
					}
				}
				out.grid.insert(coord, set);
			}
		}

		// TODO sort the jobs into the heap

		out
	}
}