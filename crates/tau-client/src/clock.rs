//! Time as the link needs it: a monotonic clock that keeps counting while the machine sleeps, for timers and the
//! 45-second rule, and the wall clock for times shown to the user.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Milliseconds on a monotonic clock that counts sleep. `std::time::Instant` stops while a Mac sleeps, so a stream
/// silent through a sleep would look fresh; this clock does not.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Mono(pub u64);

/// Unix epoch milliseconds.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Wall(pub i64);

/// One reading of both clocks. The engine never reads a clock; every input carries one of these.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Now {
    pub mono: Mono,
    pub wall: Wall,
}

impl Mono {
    pub fn after(self, d: Duration) -> Mono {
        Mono(self.0.saturating_add(d.as_millis() as u64))
    }

    /// How long from `earlier` to `self`, zero if `earlier` is later.
    pub fn since(self, earlier: Mono) -> Duration {
        Duration::from_millis(self.0.saturating_sub(earlier.0))
    }
}

impl Now {
    pub fn after(self, d: Duration) -> Now {
        Now {
            mono: self.mono.after(d),
            wall: Wall(self.wall.0 + d.as_millis() as i64),
        }
    }
}

/// Where the runtime reads the time. Tests and headless renders use [`ManualClock`].
pub trait Clock: Send + Sync {
    fn now(&self) -> Now;
}

/// The machine's clocks: `CLOCK_MONOTONIC` on macOS (it counts sleep, unlike `CLOCK_UPTIME_RAW` under `Instant`),
/// `CLOCK_BOOTTIME` on Linux.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Now {
        Now {
            mono: Mono(sleep_counting_ms()),
            wall: Wall(wall_ms()),
        }
    }
}

fn sleep_counting_ms() -> u64 {
    #[cfg(target_os = "linux")]
    let id = libc::CLOCK_BOOTTIME;
    #[cfg(not(target_os = "linux"))]
    let id = libc::CLOCK_MONOTONIC;
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` is a valid, writable timespec, and `id` is a clock this platform defines.
    let rc = unsafe { libc::clock_gettime(id, &mut ts) };
    assert_eq!(rc, 0, "clock_gettime failed");
    ts.tv_sec as u64 * 1000 + ts.tv_nsec as u64 / 1_000_000
}

fn wall_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

/// A clock that moves only when told to.
#[derive(Clone)]
pub struct ManualClock {
    mono: Arc<AtomicU64>,
    wall_start: i64,
}

impl ManualClock {
    pub fn new(start: Now) -> Self {
        Self {
            mono: Arc::new(AtomicU64::new(start.mono.0)),
            wall_start: start.wall.0 - start.mono.0 as i64,
        }
    }

    pub fn advance(&self, d: Duration) {
        self.mono.fetch_add(d.as_millis() as u64, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Now {
        let mono = self.mono.load(Ordering::SeqCst);
        Now {
            mono: Mono(mono),
            wall: Wall(self.wall_start + mono as i64),
        }
    }
}
