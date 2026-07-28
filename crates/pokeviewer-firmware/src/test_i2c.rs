//! Recording I²C bus used by target-independent hardware-protocol tests.

use core::{
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
};

extern crate std;

use embedded_hal_async::i2c::{Error, ErrorKind, ErrorType, I2c, Operation};
use std::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TestI2cError {
    Injected,
}

impl Error for TestI2cError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

pub(crate) struct RecordingI2c {
    registers: [u8; 256],
    pointer: u8,
    attempts: usize,
    fail_attempts: Vec<usize>,
    pub(crate) attempted_writes: Vec<(u8, Vec<u8>)>,
    pub(crate) register_writes: Vec<(u8, u8, u8)>,
}

impl RecordingI2c {
    pub(crate) fn new() -> Self {
        Self {
            registers: [0; 256],
            pointer: 0,
            attempts: 0,
            fail_attempts: Vec::new(),
            attempted_writes: Vec::new(),
            register_writes: Vec::new(),
        }
    }

    pub(crate) fn with_fail_attempts(mut self, attempts: &[usize]) -> Self {
        self.fail_attempts.extend_from_slice(attempts);
        self
    }

    pub(crate) fn set_register(&mut self, register: u8, value: u8) {
        self.registers[usize::from(register)] = value;
    }

    pub(crate) fn register(&self, register: u8) -> u8 {
        self.registers[usize::from(register)]
    }

    pub(crate) fn attempts(&self) -> usize {
        self.attempts
    }
}

impl ErrorType for RecordingI2c {
    type Error = TestI2cError;
}

impl I2c for RecordingI2c {
    async fn transaction(
        &mut self,
        address: u8,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        self.attempts += 1;
        for operation in operations.iter() {
            if let Operation::Write(bytes) = operation {
                self.attempted_writes.push((address, bytes.to_vec()));
            }
        }
        if self.fail_attempts.contains(&self.attempts) {
            return Err(TestI2cError::Injected);
        }

        for operation in operations {
            match operation {
                Operation::Write(bytes) if bytes.len() == 1 => {
                    self.pointer = bytes[0];
                }
                Operation::Write(bytes) if !bytes.is_empty() => {
                    self.pointer = bytes[0];
                    for &value in &bytes[1..] {
                        self.registers[usize::from(self.pointer)] = value;
                        self.register_writes.push((address, self.pointer, value));
                        self.pointer = self.pointer.wrapping_add(1);
                    }
                }
                Operation::Write(_) => {}
                Operation::Read(bytes) => {
                    for byte in bytes.iter_mut() {
                        *byte = self.registers[usize::from(self.pointer)];
                        self.pointer = self.pointer.wrapping_add(1);
                    }
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn block_on_ready<Output>(future: impl Future<Output = Output>) -> Output {
    let mut context = Context::from_waker(Waker::noop());
    let mut future = pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("recording I2C operation unexpectedly yielded"),
    }
}
