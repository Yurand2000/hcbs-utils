use crate::prelude::*;

pub mod prelude {
    pub use eva_rt_common::time::{
        Time,
        Time2,
    };

    pub use super::{
        Clock,
        clock_gettime,
        clock_sleep_absolute,
        clock_sleep_relative,
        time_to_timespec,
        timespec_to_time,
    };
}

#[derive(Debug)]
#[derive(Clone, Copy)]
pub enum Clock {
    Monotonic,
    Realtime,
}

impl Into<libc::clockid_t> for Clock {
    fn into(self) -> libc::clockid_t {
        match self {
            Clock::Monotonic => libc::CLOCK_MONOTONIC,
            Clock::Realtime => libc::CLOCK_REALTIME,
        }
    }
}

pub fn clock_gettime(clock: Clock) -> Time {
    let mut timespec = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    unsafe {
        libc::clock_gettime(
            clock.into(),
            &mut timespec
        );
    }

    timespec_to_time(timespec)
}

pub fn clock_sleep_relative(clock: Clock, time: Time) {
    let timespec = time_to_timespec(time);

    unsafe {
        libc::clock_nanosleep(
            clock.into(),
            0,
            &timespec,
            std::ptr::null_mut()
        );
    };
}

pub fn clock_sleep_absolute(clock: Clock, time: Time) {
    let timespec = time_to_timespec(time);

    unsafe {
        libc::clock_nanosleep(
            clock.into(),
            libc::TIMER_ABSTIME,
            &timespec,
            std::ptr::null_mut()
        );
    };
}

pub fn timespec_to_time(timespec: libc::timespec) -> Time {
    Time::secs(timespec.tv_sec as f64) +
        Time::nanos(timespec.tv_nsec as f64)
}

pub fn time_to_timespec(time: Time) -> libc::timespec {
    libc::timespec {
        tv_sec: (time.as_secs().floor()) as i64,
        tv_nsec: (time.as_secs().fract() * Time::SECS_TO_NANO) as i64,
    }
}