extern crate itertools;
extern crate kdtree;

use util::{Coord, TimeStep, FileReader, FileWriter, cmp_i32};
use vehicle::Vehicle;
use std::collections::{HashMap};
use std::rc::Rc;
use std::cmp::Ordering;
use std::vec::Vec;
use std::cell::RefCell;
use self::itertools::Itertools;
use self::kdtree::KdTree;
use std::hash::{Hash, Hasher};

pub type JobId = i32;

pub trait HasJob {
	fn id(&self) -> JobId;
	fn start(&self) -> Coord;
	fn end(&self) -> Coord;
	fn earliest_start(&self) -> TimeStep;
	fn latest_finish(&self) -> TimeStep;
	fn dist(&self) -> i32;
}

pub type JobPtr = Rc<HasJob>;

impl PartialOrd for HasJob {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(cmp_i32(self.earliest_start(), other.earliest_start()))
	}
}
impl PartialEq for HasJob {
	fn eq(&self, other: &Self) -> bool {
		cmp_i32(self.earliest_start(), other.earliest_start()) == Ordering::Equal
	}
}
impl Ord for HasJob {
	fn cmp(&self, other: &Self) -> Ordering {
		cmp_i32(self.earliest_start(), other.earliest_start())
	}
}
impl Eq for HasJob {}
impl Hash for HasJob {
	fn hash<H: Hasher>(&self, state: &mut H) {
		state.write_i32(self.id());
		state.finish();
	}
}

#[derive(Debug, Default)]
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
	fn dist(&self) -> i32 {
		self.start.dist(&self.end)
	}
}

type VehPtr = Rc<RefCell<Vehicle>>;

/// Output of a single simulation timestep
pub enum TickComplete {
	/// No-op, nothing to report
	Continue,
	/// Vehicle began moving from the start
	JobStart(JobPtr),
	/// Assign new job
	JobComplete(JobPtr, Coord),
}

pub struct JobScheduler {
	num_rows:			i32,
	num_cols:			i32,
	num_vehicles:		i32,
	num_jobs:			i32,
	ride_bonus:			i32,
	max_tsteps:			TimeStep,

	current_step:		TimeStep,
	fleet:              Vec<VehPtr>,
	jobs:               Vec<JobPtr>,
	rem_jobs:           Vec<JobPtr>,

	job_scores:         HashMap<JobPtr, i32>,
}

impl JobScheduler {
	pub fn new(input: FileReader) -> JobScheduler  {
		let mut out = JobScheduler {
			num_rows: 0,
			num_cols: 0,
			num_vehicles: 0,
			num_jobs: 0,
			ride_bonus: 0,
			max_tsteps: 0,
			current_step: 0,
			fleet: Vec::default(),
			jobs: Vec::default(),
			rem_jobs: Vec::default(),
			job_scores: HashMap::default(),
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
						out.num_jobs = splits[3];
						out.ride_bonus = splits[4];
						out.max_tsteps = splits[5];

						out.current_step = 0;
						out.fleet = Vec::with_capacity(out.num_vehicles as usize);
						out.jobs = Vec::with_capacity(out.num_jobs as usize);
						out.rem_jobs = Vec::with_capacity(out.num_jobs as usize);
						out.job_scores = HashMap::with_capacity(out.num_jobs as usize);
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
			out.job_scores.insert(j.clone(), 0);
		}

		out.rem_jobs.sort_by(|a, b| {
			if a.earliest_start() < b.earliest_start() {
				Ordering::Less
			} else if a.earliest_start() > b.earliest_start() {
				Ordering::Greater
			} else {
				cmp_i32(a.id(), b.id())
			}
		});

		out
	}

	fn tick_vehicles(&mut self) -> KdTree<VehPtr, [f64; 2]> {
		let mut bounding_tree = KdTree::new(2);

		for v in self.fleet.iter_mut() {
			if self.current_step == 1 {
				// all vehicles are idle in the first tick
				let coord = v.borrow().current_pos().unwrap();
				bounding_tree.add([coord.x as f64, coord.y as f64], v.clone());
				continue;
			}

			let result = v.borrow_mut().tick(self.current_step);
			match result {
				TickComplete::Continue => {},
				TickComplete::JobStart(job) => {
					// save the negative score for easy exclusion later
					let score = -(job.dist() + (self.ride_bonus * (self.current_step == job.earliest_start()) as i32));
					self.job_scores.insert(job.clone(), score);
				}
				TickComplete::JobComplete(job, coord) => {
					// flip the sign on the score if it arrives on time
					if self.current_step < job.latest_finish() {
						let score = self.job_scores[&job];
						self.job_scores.insert(job.clone(), -score);
					}

					bounding_tree.add([coord.x as f64, coord.y as f64], v.clone());
//					println!("Vehicle {} completed job", v.borrow().id());
				},
			};
		}

		bounding_tree
	}

	fn greedy_scheduling(&mut self, idle_vehicles: &KdTree<VehPtr, [f64; 2]>) {
		if idle_vehicles.size() > 0 {
			'assign_loop: loop {
				let mut assigned = false;

				match self.rem_jobs.last() {
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
									v.borrow_mut().queue_new_job(&j);
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
	}

	fn funky_scheduling(&mut self, idle_vehicles: &KdTree<VehPtr, [f64; 2]>) {
		if idle_vehicles.size() > 0 {
			let mut candidates: Vec<VehPtr> = Vec::new();
			let mut relax_start = false;
			let mut relax_end = false;

			'assign_loop: loop {
				let mut assigned_idx: i32 = -1;
				candidates.clear();

				'job_loop: for (idx, j) in self.rem_jobs.iter().enumerate() {
					let start = j.start();
					let dist_measure = |a: &[f64], b: &[f64]| {
						a.iter().zip(b.iter())
							.map(|(x, y)| f64::abs(x - y))
							.fold(0f64, ::std::ops::Add::add)
					};

//					println!("Assigning job {} | start {},{} | end {},{} | earliest {}",
//					        j.id(),
//							j.start().x, j.start().y,
//							j.end().x, j.end().y,
//							j.earliest_start());

					'nearest_loop: for mut itr in idle_vehicles.iter_nearest(vec![start.x as f64, start.y as f64].as_slice(), &dist_measure) {
						while let Some(&mut (dist_from_start, v)) = itr.next().as_mut() {
							if v.borrow().is_idle() {
								candidates.push(v.clone());
//								println!("Candidate {} for Job {}", v.borrow().id(), j.id());

								let pos = v.borrow().current_pos().unwrap();
								let dist_to_start = pos.dist(&j.start());
								let tot_dist = dist_to_start + j.dist();

								if relax_end || self.current_step + tot_dist < j.latest_finish() {
									if relax_start || self.current_step + dist_to_start < j.earliest_start() {
//										println!("\tAssigned to vehicle {}", v.borrow().id());

										v.borrow_mut().queue_new_job(&j);
										assigned_idx = idx as i32;
										break 'job_loop;
									}
								}

							}
						}
					}
				}

				if assigned_idx != -1 {
					self.rem_jobs.remove(assigned_idx as usize);
					relax_start = false;
					relax_end = false;
				} else if !candidates.is_empty() {
					// relax conditions one by one
					if !relax_start || !relax_end {
						if !relax_start {
							relax_start = true;
						} else {
							relax_end = true;
						}
					} else {
						unreachable!();
					}

				} else {
					break 'assign_loop;
				}
			}
		}
	}

	pub fn run(&mut self) {
		assert_eq!(self.rem_jobs.is_empty(), false);

		println!("Being Simulation | Vehicles: {} | Jobs: {} | Ticks: {}",
		         self.num_vehicles,
		         self.num_jobs,
		         self.max_tsteps);

		'sim_loop: for step in 1..self.max_tsteps {
			self.current_step = step;

			let idle_vehicles = self.tick_vehicles();
			self.funky_scheduling(&idle_vehicles);
		}

		println!("End Simulation | Remaining jobs: {} | Idling Vehicles: {} | Score: {}",
		         self.rem_jobs.len(),
		         self.fleet.iter().filter(|v| v.borrow().is_idle()).collect_vec().len(),
				 self.calculate_score());
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

	pub fn calculate_score(&self) -> u64 {
		self.job_scores.values().into_iter().fold(0, |a, s| if *s > 0 { a + *s as u64 } else { a } )
	}
}