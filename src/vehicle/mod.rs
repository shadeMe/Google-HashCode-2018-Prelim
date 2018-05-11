extern crate itertools;

use scheduler::{Job, JobId};
use scheduler::TickComplete;
use std::hash::{Hash, Hasher};
use util::{Coord, TimeStep};

#[derive(PartialEq, Copy, Clone, Hash, Eq, Debug)]
enum RideTaskType {
    Invalid,
    DrivingToStart,
    WaitingAtStart,
    DrivingToEnd,
}

struct RideTask {
    task_type: RideTaskType,
    rem_steps: TimeStep,
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
        // weak hash, but we don't care as the task object is meant to be transient
        state.write_u32(self.task_type as u32);
        state.write_i32(self.rem_steps);
        state.finish();
    }
}

impl RideTask {
    fn new(task_type: RideTaskType, rem_steps: TimeStep) -> RideTask {
        RideTask {
            task_type,
            rem_steps,
        }
    }

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

    fn task_type(&self) -> RideTaskType {
        self.task_type
    }
}

type VehicleId = i32;

#[derive(Eq)]
pub struct Vehicle {
    id: VehicleId,
    jobs: Vec<Job>,
    ride_tasks: Vec<RideTask>,
    job_buffer: Option<Job>,
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
        Vehicle {
            id,
            jobs: Vec::<Job>::new(),
            ride_tasks: Vec::<RideTask>::new(),
            job_buffer: None,
        }
    }

    pub fn id(&self) -> VehicleId {
        self.id
    }

    fn current_task_mut(&mut self) -> Option<&mut RideTask> {
        match self.ride_tasks.len() {
            l if l > 0 => Some(&mut self.ride_tasks[l - 1]),
            _ => None,
        }
    }

    fn current_task(&self) -> Option<&RideTask> {
        self.ride_tasks.last()
    }

    fn current_job(&self) -> Option<&Job> {
        self.jobs.last()
    }

    pub fn current_pos(&self) -> Option<Coord> {
        match self.current_task() {
            Some(t) => {
                if t.is_idle() {
                    match t.task_type() {
                        RideTaskType::DrivingToStart | RideTaskType::WaitingAtStart => {
                            return Some(self.current_job().unwrap().start());
                        }
                        RideTaskType::DrivingToEnd => {
                            return Some(self.current_job().unwrap().end());
                        }
                        _ => unreachable!(),
                    };
                } else {
                    // no position when in transit
                    return None;
                }
            }
            None => Some(Coord::default()), // origin if at start
        }
    }

    pub fn is_idle(&self) -> bool {
        self.job_buffer.is_none()
            && (self.ride_tasks.len() == 0 || self.current_task().unwrap().has_arrived_at_dest())
    }

    fn add_job_task(
        &mut self,
        job_start: Coord,
        job_end: Coord,
        job_earliest_start: TimeStep,
        current_step: TimeStep,
    ) -> RideTaskType {
        let mut task_type = RideTaskType::Invalid;
        let mut task_steps = 0;

        let cur_pos = self.current_pos().unwrap();
        let dist_to_end = cur_pos.dist(&job_end);
        let dist_to_start = cur_pos.dist(&job_start);

        assert!(dist_to_start >= 0 && dist_to_end >= 0);
        {
            let mut set_task_params = |t, s| {
                task_type = t;
                task_steps = s;
                //println!("Vehicle {} -> Task {:?}, Steps {}", self.id(), t, s);
            };

            if dist_to_start > 0 {
                set_task_params(RideTaskType::DrivingToStart, dist_to_start);
            } else if dist_to_start == 0 {
                if current_step >= job_earliest_start {
                    set_task_params(RideTaskType::DrivingToEnd, dist_to_end);
                } else {
                    set_task_params(
                        RideTaskType::WaitingAtStart,
                        job_earliest_start - current_step,
                    );
                }
            } else if dist_to_end > 0 {
                set_task_params(RideTaskType::DrivingToEnd, dist_to_end);
            } else {
                // not sure if this ever happens in the input data (start_pos == end_pos for some pair of jobs)
                unreachable!();
            }
        }

        self.ride_tasks.push(RideTask::new(task_type, task_steps));
        task_type
    }

    pub fn queue_new_job(&mut self, job: Job) {
        assert_eq!(self.job_buffer.is_none(), true);
        self.job_buffer = Some(job);
    }

    pub fn tick(&mut self, current_step: TimeStep) -> TickComplete {
        // load the job on the buffer, if any
        if let Some(new_jerb) = self.job_buffer.take() {
            self.add_job_task(
                new_jerb.start(),
                new_jerb.end(),
                new_jerb.earliest_start(),
                current_step,
            );
            self.jobs.push(new_jerb);
        }

        assert!(self.ride_tasks.len() > 0);
        let mut out = TickComplete::Continue;
        let current_job_id = self.current_job().map(|j| j.id());
        let current_job_start = self.current_job().map(|j| j.start());
        let current_job_end = self.current_job().map(|j| j.end());
        let current_job_dist = self.current_job().map(|j| j.dist());
        let current_job_earliest_start = self.current_job().map(|j| j.earliest_start());
        let current_job_latest_finish = self.current_job().map(|j| j.latest_finish());

        if let Some(t) = self.current_task_mut() {
            if !t.is_idle() {
                if t.step() {
                    if t.has_arrived_at_start() {
                        // the job task will be updated next tick
                        out = TickComplete::Continue;
                    } else if t.is_done_waiting() {
                        // same as above, the next job task will be updated next tick
                        out = TickComplete::JobStart(
                            current_job_id.unwrap(),
                            current_job_dist.unwrap(),
                            current_job_earliest_start.unwrap(),
                        );
                    } else if t.has_arrived_at_dest() {
                        out = TickComplete::JobComplete(
                            current_job_id.unwrap(),
                            current_job_latest_finish.unwrap(),
                            current_job_end.unwrap(),
                        );
                    } else {
                        unreachable!()
                    }
                }
            }
        }

        // strange control flow because borrow checker (tm)
        if let Some(true) = self.current_task()
            .map(|t| t.is_idle() && !t.has_arrived_at_dest())
            {
                assert_eq!(self.job_buffer, None);

                // account for cases where the waiting state is not available/is skipped
                match self.add_job_task(
                    current_job_start.unwrap(),
                    current_job_end.unwrap(),
                    current_job_earliest_start.unwrap(),
                    current_step,
                ) {
                    RideTaskType::DrivingToEnd => {
                        out = TickComplete::JobStart(
                            current_job_id.unwrap(),
                            current_job_dist.unwrap(),
                            current_job_earliest_start.unwrap(),
                        );
                    }
                    _ => {}
                };
            }

        out
    }

    pub fn assigned_rides(&self) -> Vec<JobId> {
        self.jobs.iter().map(|t| t.id()).collect()
    }
}
