use util::{Coord, TimeStep};
use scheduler::{JobPtr, HasJob};
use scheduler::{NewTick, NewJob, TickComplete};
use std::hash::{Hash, Hasher};

#[derive(PartialEq, Copy, Clone, Hash, Eq)]
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
	fn eq(&self, other: &RideTask) -> bool {
		// all tasks are unique
		false
	}
}

impl Hash for RideTask {
	fn hash<H: Hasher>(&self, state: &mut H) {
		// FIXME weak hash
		state.write_u32(self.task_type as u32);
		state.write_i32(self.rem_steps);
		state.finish();
	}
}

impl RideTask {
	fn step(&mut self) -> () {
		assert!(self.task_type != RideTaskType::Invalid);
		assert!(self.rem_steps > 0);

		match self.task_type {
			RideTaskType::DrivingToStart | RideTaskType::DrivingToEnd => self.rem_steps -= 1,
			_ => {}
		};
	}

	fn has_arrived_at_start(&self) -> bool {
		self.task_type == RideTaskType::DrivingToStart && self.rem_steps == 0
	}

	fn is_waiting_at_start(&self) -> bool {
		self.task_type == RideTaskType::WaitingAtStart
	}

	fn has_arrived_at_dest(&self) -> bool {
		self.task_type == RideTaskType::DrivingToEnd && self.rem_steps == 0
	}

	fn job(&self) -> JobPtr {
		self.parent.clone()
	}

	fn job_if_complete(&self) -> Option<JobPtr> {
		match self.has_arrived_at_dest() {
			true => Some(self.job()),
			_ => None
		}
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
			id: id,
			rides: Vec::<RideTask>::new(),
		}
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

	pub fn get_current_pos(&self) -> Option<Coord> {
		self.current_task().and_then(|t| t.job_if_complete())
			.and_then(|j| Some(j.end()))
	}

	pub fn new_job(&mut self, context: NewJob) -> () {
		assert!(self.rides.len() == 0 || self.current_task().unwrap().has_arrived_at_dest());

		let mut task_builder = RideTaskBuilder::new(&context.job);
		let job = &context.job;
		task_builder.set_parent(&job);

		let cur_pos = self.get_current_pos().unwrap();
		let dist_to_end = cur_pos.dist(&job.end());
		let dist_to_start = cur_pos.dist(&job.start());

		{
			let mut set_task_params = |t, s| {
				task_builder.set_task_type(t);
				task_builder.set_rem_steps(s);
			};

			if dist_to_start > 0 {
				set_task_params(RideTaskType::DrivingToStart, dist_to_start);
			}
				else if dist_to_start == 0 {
					if context.current_step >= job.earliest_start() {
						set_task_params(RideTaskType::DrivingToEnd, dist_to_end);
					}
						else {
							set_task_params(RideTaskType::WaitingAtStart, 0);
						}
				}
					else if dist_to_end > 0 {
						set_task_params(RideTaskType::DrivingToEnd, dist_to_end);
					}
						else {
							// not sure if this ever happens in the input data (start_pos == end_pos for some pair of jobs)
							unreachable!()
						}
		}

		self.rides.push(task_builder.build())
	}

	pub fn tick(&mut self, context: NewTick) -> TickComplete {
		if let Some(t) = self.current_task_mut() {
			t.step();
			let job = t.job();

			if t.has_arrived_at_start() {
				TickComplete::Reschedule(job.clone());
			}
				else if t.is_waiting_at_start() {
					if context.current_step >= job.earliest_start() {
						TickComplete::Reschedule(job.clone());
					}
				}
					else if t.has_arrived_at_dest() {
						TickComplete::JobComplete(job.end());
					}
		}

		TickComplete::Continue
	}
}
