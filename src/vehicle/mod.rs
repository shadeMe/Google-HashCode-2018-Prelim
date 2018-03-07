extern crate itertools;

use util::{Coord, TimeStep};
use scheduler::{JobPtr, JobId};
use scheduler::{NewTick, NewJob, TickComplete};
use std::hash::{Hash, Hasher};
use self::itertools::Itertools;

#[derive(PartialEq, Copy, Clone, Hash, Eq, Debug)]
enum RideTaskType {
	Invalid,
	DrivingToStart,
	WaitingAtStart,
	DrivingToEnd
}

struct RideTask {
	task_type:		RideTaskType,
	parent:			JobPtr,
	rem_steps:		TimeStep,
}

impl Eq for RideTask {}

impl PartialEq for RideTask {
	fn eq(&self, _other: &RideTask) -> bool {
		// all tasks are unique
		false
	}
}

impl Hash for RideTask {
	fn hash<H: Hasher>(&self, state: &mut H) {
		// weak hash, but we don't care as the task object is mean to be transient
		state.write_u32(self.task_type as u32);
		state.write_i32(self.rem_steps);
		state.finish();
	}
}

impl RideTask {
	fn step(&mut self) -> bool {
		assert!(self.task_type != RideTaskType::Invalid);
		assert!(self.is_idle() == false);

		self.rem_steps -= 1;
		self.is_idle()
	}

	fn is_idle(&self) -> bool {
		self.rem_steps == 0
	}

	fn has_arrived_at_start(&self) -> bool {
		self.task_type == RideTaskType::DrivingToStart && self.is_idle()
	}

	fn is_done_waiting(&self) -> bool {
		self.task_type == RideTaskType::WaitingAtStart && self.is_idle()
	}

	fn has_arrived_at_dest(&self) -> bool {
		self.task_type == RideTaskType::DrivingToEnd && self.is_idle()
	}

	fn job(&self) -> JobPtr {
		self.parent.clone()
	}

	fn job_id(&self) -> JobId {
		self.parent.id()
	}

	fn task_type(&self) -> RideTaskType {
		self.task_type
	}
}

struct RideTaskBuilder {
	buf:		RideTask,
}

impl RideTaskBuilder {
	fn new(j : &JobPtr) -> RideTaskBuilder {
		RideTaskBuilder{
			buf: RideTask {
					task_type: RideTaskType::Invalid,
					parent: j.clone(),
					rem_steps: 0
			}
		}
	}

	fn set_task_type(&mut self, t: RideTaskType) {
		self.buf.task_type = t;
	}

	fn set_parent(&mut self, p: &JobPtr) {
		self.buf.parent = p.clone();
	}

	fn set_rem_steps(&mut self, s: TimeStep) {
		self.buf.rem_steps = s;
	}

	fn build(&mut self) -> RideTask {
		assert!(self.buf.task_type != RideTaskType::Invalid);

		RideTask {
			task_type: self.buf.task_type,
			parent: self.buf.parent.clone(),
			rem_steps: self.buf.rem_steps
		}
	}
}

type VehicleId = i32;

#[derive(Eq)]
pub struct Vehicle {
	id:			VehicleId,
	rides:		Vec<RideTask>
}

impl PartialEq for Vehicle {
	fn eq(&self, other: &Vehicle) -> bool {
		self.id == other.id
	}
}

impl Hash for Vehicle {
	fn hash<H: Hasher>(&self, state: &mut H) {
		state.write_i32(self.id);
		state.finish();
	}
}

impl Vehicle {
	pub fn new(id: VehicleId) -> Vehicle {
		Vehicle{
			id,
			rides: Vec::<RideTask>::new(),
		}
	}

	pub fn id(&self) -> VehicleId {
		self.id
	}

	fn current_task_mut(&mut self) -> Option<&mut RideTask> {
		match self.rides.len() {
			l if l > 0 => Some(&mut self.rides[l - 1]),
			_ => None
		}
	}

	fn current_task(&self) -> Option<&RideTask> {
		self.rides.last()
	}

	pub fn current_pos(&self) -> Option<Coord> {
		match self.current_task() {
			Some(t) => {
				if t.is_idle() {
					match t.task_type() {
						RideTaskType::DrivingToStart | RideTaskType::WaitingAtStart => {
							return Some(t.job().start());
						},
						RideTaskType::DrivingToEnd => {
							return Some(t.job().end());
						},
						_ => {
							unreachable!();
						}
					};
				}
				else {
					// no position when in transit
					return None;
				}
			},
			None => Some(Coord::default())        // origin if at start
		}
	}

	pub fn is_idle(&self) -> bool {
		self.rides.len() == 0 || self.current_task().unwrap().has_arrived_at_dest()
	}

	pub fn new_job(&mut self, context: NewJob) -> () {
		let mut task_builder = RideTaskBuilder::new(&context.job);
		let job = &context.job;
		task_builder.set_parent(&job);

		let cur_pos = self.current_pos().unwrap();
		let dist_to_end = cur_pos.dist(&job.end());
		let dist_to_start = cur_pos.dist(&job.start());
		assert!(dist_to_start >= 0 && dist_to_end >= 0);

		{
			let mut set_task_params = |t, s| {
				task_builder.set_task_type(t);
				task_builder.set_rem_steps(s);
		//		println!("Vehicle {} -> Task {:?}, Steps {}", self.id(), t, s);
			};

			if dist_to_start > 0 {
				set_task_params(RideTaskType::DrivingToStart, dist_to_start);
			} else if dist_to_start == 0 {
				if context.current_step >= job.earliest_start() {
					set_task_params(RideTaskType::DrivingToEnd, dist_to_end);
				} else {
					set_task_params(RideTaskType::WaitingAtStart, job.earliest_start() - context.current_step);
				}
			} else if dist_to_end > 0 {
					set_task_params(RideTaskType::DrivingToEnd, dist_to_end);
			} else {
				// not sure if this ever happens in the input data (start_pos == end_pos for some pair of jobs)
				unreachable!();
			}
		}

		self.rides.push(task_builder.build())
	}

	pub fn tick(&mut self, _context: &NewTick) -> TickComplete {
		let current_task = self.current_task_mut();
		match current_task {
			Some(t) => {
				if !t.is_idle() {
					if t.step() {
						let job = t.job();

						if t.has_arrived_at_start() {
							return TickComplete::Reschedule(job.clone());
						} else if t.is_done_waiting() {
							return TickComplete::Reschedule(job.clone());
						} else if t.has_arrived_at_dest() {
							return TickComplete::JobComplete(job.end());
						} else {
							unreachable!();
						}
					}
				}

				TickComplete::Continue
			},
			None => {
				unreachable!();
			}
		}
	}

	pub fn assigned_rides(&self) -> Vec<JobId> {
		self.rides.iter().map(|t| t.job_id()).into_iter().dedup().collect()
	}
}
