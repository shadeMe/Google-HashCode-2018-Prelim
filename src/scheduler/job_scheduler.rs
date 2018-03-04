use util::{Coord, TimeStep};
use scheduler::job::JobPtr;

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
	num_cars:			i32,
	num_rides:			i32,
	ride_bonus:			i32,
	max_tsteps:			TimeStep,
	
	current_step:		TimeStep,
	
}

impl JobScheduler {
	
}