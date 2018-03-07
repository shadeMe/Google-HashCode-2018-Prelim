extern crate itertools;
extern crate kdtree;

use util::{Coord, TimeStep, FileReader, FileWriter, cmp_i32};
use vehicle::Vehicle;
use std::collections::{BinaryHeap};
use std::rc::Rc;
use std::cmp::Ordering;
use std::vec::Vec;
use std::cell::RefCell;
use self::itertools::Itertools;
use self::kdtree::KdTree;

pub type JobId = i32;

pub trait HasJob {
	fn id(&self) -> JobId;
	fn start(&self) -> Coord;
	fn end(&self) -> Coord;
	fn earliest_start(&self) -> TimeStep;
	fn latest_finish(&self) -> TimeStep;
}

pub type JobPtr = Rc<HasJob>;

impl PartialOrd for HasJob {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(cmp_i32(other.earliest_start(), self.earliest_start()))
	}
}
impl PartialEq for HasJob {
	fn eq(&self, other: &Self) -> bool {
		cmp_i32(other.earliest_start(), self.earliest_start()) == Ordering::Equal
	}
}
impl Ord for HasJob {
	fn cmp(&self, other: &Self) -> Ordering {
		// We flip the ordering as we need a min-heap
		cmp_i32(other.earliest_start(), self.earliest_start())
	}
}
impl Eq for HasJob {}

#[derive(Debug)]
pub struct Job {
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

type VehPtr = Rc<RefCell<Vehicle>>;

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

pub struct JobScheduler {
	num_rows:			i32,
	num_cols:			i32,
	num_vehicles:		i32,
	num_rides:			i32,
	ride_bonus:			i32,
	max_tsteps:			TimeStep,

	current_step:		TimeStep,
	fleet:              Vec<VehPtr>,
	jobs:               Vec<JobPtr>,
	rem_jobs:           BinaryHeap<JobPtr>,
}

impl JobScheduler {
	pub fn new(input: FileReader) -> JobScheduler  {
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
			rem_jobs: BinaryHeap::default(),
		};

		// parse input
		match input.read_all_lines() {
			Ok(lines) => {
				for (line_no, line) in lines.iter().enumerate() {
					let mut splits: Vec<i32> = line.split(' ').map(|s| s.parse::<i32>().unwrap()).collect();
					assert_eq!(splits.len(), 6);

					if line_no == 0 {
						out.num_rows = splits[0];
						out.num_cols = splits[1];
						out.num_vehicles = splits[2];
						out.num_rides = splits[3];
						out.ride_bonus = splits[4];
						out.max_tsteps = splits[5];

						out.current_step = 0;
						out.fleet = Vec::with_capacity(out.num_vehicles as usize);
						out.jobs = Vec::with_capacity(out.num_rides as usize);
						out.rem_jobs = BinaryHeap::with_capacity(out.num_rides as usize);
					}
					else {
						let adjusted_line_no : i32 = line_no as i32 - 1;      // ride numbers start at 0
						let x_start = splits[0];
						let y_start = splits[1];
						let x_end = splits[2];
						let y_end = splits[3];
						let early = splits[4];
						let late = splits[5];

						let jerb = Job{
							id: adjusted_line_no,
							start: Coord::new(x_start, y_start),
							end: Coord::new(x_end, y_end),
							earliest_start: early,
							latest_end: late,
						};

						out.jobs.push(Rc::new(jerb));
					}

				}
			},
			Err(errs) => errs.into_iter().foreach(|err| print!("Error reading input. Error: {:?}", err)),
		};

		for i in 0..out.num_vehicles {
			out.fleet.push(Rc::new(RefCell::new(Vehicle::new(i))));
		}

		for j in &out.jobs {
			out.rem_jobs.push(j.clone());
		}

		out
	}

	fn tick_vehicles(&mut self) -> KdTree<VehPtr, [f64; 2]> {
		let tick_context = NewTick{current_step: self.current_step};
		let mut bounding_tree = KdTree::new(2);

		for v in self.fleet.iter_mut() {
			if self.current_step == 1 {
				// all vehicles are idle in the first tick
				let coord = v.borrow().current_pos().unwrap();
				bounding_tree.add([coord.x as f64, coord.y as f64], v.clone());
				continue;
			}

			let result = v.borrow_mut().tick(&tick_context);
			match result {
				TickComplete::Continue => {},
				TickComplete::Reschedule(job) => {
					v.borrow_mut().new_job(NewJob { job, current_step: self.current_step });
				}
				TickComplete::JobComplete(coord) => {
//					println!("Vehicle {} completed job", v.borrow().id());
					bounding_tree.add([coord.x as f64, coord.y as f64], v.clone());
				},
			};
		}

		bounding_tree
	}

	pub fn run(&mut self) {
		assert_eq!(self.rem_jobs.is_empty(), false);

		println!("Being simulation | Vehicles: {} | Jobs: {} | Ticks: {}",
				self.num_vehicles,
				self.num_rides,
				self.max_tsteps);

		'sim_loop: for step in 1..self.max_tsteps {
			self.current_step = step;

			let idle_vehicles = self.tick_vehicles();
			if idle_vehicles.size() > 0 {
				'assign_loop: loop {
					let mut assigned = false;

					match self.rem_jobs.peek() {
						None => break 'assign_loop,
						Some(j) => {
							// greedy solution
							let start = j.start();
							let end = j.end();
							let earliest_start_delta = j.earliest_start() - self.current_step;
							let dist_measure = |a: &[f64], b: &[f64]| {
								a.iter().zip(b.iter())
									.map(|(x, y)| f64::abs(x - y))
									.fold(0f64, ::std::ops::Add::add)
							};

//							println!("Assigning job {} | start {},{} | end {},{} | earliest {}",
//							        j.id(),
//									j.start().x, j.start().y,
//									j.end().x, j.end().y,
//									j.earliest_start());

							'nearest_loop: for mut itr in idle_vehicles.iter_nearest(vec![start.x as f64, start.y as f64].as_slice(), &dist_measure) {
								while let Some(&mut (dist_from_start, v)) = itr.next().as_mut() {
									if v.borrow().is_idle() {
										v.borrow_mut().new_job(NewJob { job: j.clone(), current_step: self.current_step });
//										println!("\tAssigned to vehicle {}", v.borrow().id());

										assigned = true;
										break 'nearest_loop;
									}
								}
							}
						}
					}

					if assigned {
						self.rem_jobs.pop();
					} else {
						break 'assign_loop;
					}
				}
			}

//			if step % 100 == 0 {
//				println!("End of step {}/{} | Remaining jobs: {}", step, self.max_tsteps, self.rem_jobs.len());
//			}
		}

		println!("Simulation ended | Remaining jobs: {} | Idling Vehicles: {}",
		         self.rem_jobs.len(),
		         self.fleet.iter().filter(|v| v.borrow().is_idle()).collect_vec().len());
	}

	pub fn write_output(&self, out: &mut FileWriter) {
		out.write_line(&self.output_as_str());
	}

	pub fn output_as_str(&self) -> String {
		let mut out = String::new();
		for (idx, v) in self.fleet.iter().enumerate() {
			let v = v.borrow();
			assert_eq!(v.id(), idx as i32);

			let rides = v.assigned_rides();
			out += &format!("{}{}\n", rides.len(), rides.iter().fold(String::new(), |s, id| s + " " + id.to_string().as_str()).as_str());
		}

		out
	}
}