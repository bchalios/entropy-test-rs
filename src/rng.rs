use nix::ioctl_read_bad;
use nix::libc::c_int;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::os::unix::io::AsRawFd;

use rand::RngCore;

use crate::bindings::RNDGETENTCNT;

ioctl_read_bad!(get_entropy_count, RNDGETENTCNT, c_int);

pub trait Rng {
    /// Fill the `bytes` slice with random bytes
    fn get_random(&mut self, bytes: &mut [u8]) -> Result<usize>;

    /// Returns the entropy count of the Rng
    fn get_entropy_count(&self) -> Result<u32>;
}

#[derive(Debug, thiserror::Error)]
pub enum UrandomError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("ioctl error: {0}")]
    Ioctl(#[from] nix::Error),
    #[error("rand crate error: {0}")]
    Rand(#[from] rand::Error),
}

pub type Result<T> = std::result::Result<T, UrandomError>;

pub struct DevUrandom {
    inner: File,
}

impl DevUrandom {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: File::open("/dev/urandom")?,
        })
    }
}

impl Rng for DevUrandom {
    fn get_random(&mut self, bytes: &mut [u8]) -> Result<usize> {
        Ok(self.inner.read(bytes)?)
    }

    fn get_entropy_count(&self) -> Result<u32> {
        let fd = self.inner.as_raw_fd();
        let mut entropy_count = 0;

        // SAFETY: This should be safe since we're passing a valid file descriptor,
        // i.e. the one of the `/dev/urandom` file we own.
        let _ = unsafe { get_entropy_count(fd, &mut entropy_count)? };
        Ok(entropy_count as u32)
    }
}

/// Wrapper on top of the `rand`-crate `ThreadRng`
#[derive(Clone)]
pub struct ThreadRng(rand::rngs::ThreadRng);

impl ThreadRng {
    pub fn new() -> Self {
        Self(rand::thread_rng())
    }
}

impl Rng for ThreadRng {
    fn get_random(&mut self, bytes: &mut [u8]) -> Result<usize> {
        self.0.try_fill_bytes(bytes)?;
        Ok(bytes.len())
    }

    fn get_entropy_count(&self) -> Result<u32> {
        let f = File::open("/proc/sys/kernel/random/entropy_avail").unwrap();
        Ok(io::BufReader::new(f)
            .lines()
            .next()
            .unwrap()
            .unwrap()
            .parse::<u32>()
            .unwrap())
    }
}

/// Wrapper on top of the `rand`-crate `OsRng`
#[derive(Clone)]
pub struct OsRng(rand::rngs::OsRng);

impl OsRng {
    pub fn new() -> Self {
        Self(rand::rngs::OsRng)
    }
}

impl Rng for OsRng {
    fn get_random(&mut self, bytes: &mut [u8]) -> Result<usize> {
        self.0.try_fill_bytes(bytes)?;
        Ok(bytes.len())
    }

    fn get_entropy_count(&self) -> Result<u32> {
        let f = File::open("/proc/sys/kernel/random/entropy_avail").unwrap();
        Ok(io::BufReader::new(f)
            .lines()
            .next()
            .unwrap()
            .unwrap()
            .parse::<u32>()
            .unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::{self, BufRead};

    #[test]
    fn test_open_urandom() {
        assert!(DevUrandom::new().is_ok());
    }

    #[test]
    fn test_get_random() {
        let mut urandom = DevUrandom::new().unwrap();
        let mut data = vec![0u8; 10];

        assert!(matches!(urandom.get_random(data.as_mut_slice()), Ok(_)));
    }

    #[test]
    fn test_get_entropy_count() {
        let urandom = DevUrandom::new().unwrap();
        let count = urandom.get_entropy_count().unwrap();
        // We should be able to read the same value from /proc/sys/kernel/random/entropy_avail
        // This test is *super racy* but it should succeed in a "well-behaving" system, i.e. no
        // entropy-draining events are in-flight while testing
        let f = File::open("/proc/sys/kernel/random/entropy_avail").unwrap();
        assert_eq!(
            count,
            io::BufReader::new(f)
                .lines()
                .next()
                .unwrap()
                .unwrap()
                .parse::<u32>()
                .unwrap()
        );
    }
}
