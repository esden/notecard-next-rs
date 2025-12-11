//! # blues-notecard-next
//!
//! This crate provides an asynchronous driver for the Blues Notecard.
//!
//! The driver is designed to work over the UART serial 9600 baud, the AUX UART serial 115200 baud and the i2c interface.
//!
//! # Example
//!
//! ```rust, ignore
//! #![no_std]
//! #![no_main]
//!
//! use defmt ::*;
//! use embassy_executor::Spawner
//! ...
//! ```

#![no_std]

use core::default::Default;

use chrono::Duration;

use defmt::{debug, error, trace};
use embedded_hal_async::delay::DelayNs;
use embedded_io_async::{Read, Write};
use futures::{select_biased, FutureExt};
use heapless::Vec;
use serde::{de::DeserializeOwned, Serialize};

mod error;
pub mod hub;

const CARD_RESET_DRAIN_DELAY: Duration = Duration::milliseconds(500);
const DEFAULT_BUF_SIZE: usize = 18 * 1024;
const CHUNK_LENGTH_MAX: usize = 127;
const CHUNK_LENGTH_I: usize = 30;
const CHUNK_LENGTH: usize = if CHUNK_LENGTH_I < CHUNK_LENGTH_MAX {
    CHUNK_LENGTH_I
} else {
    CHUNK_LENGTH_MAX
};

// `note-c` uses `250` for `SEGMENT_LENGTH`. Round to closest CHUNK_LENGTH
// divisible to avoid unnecessary fragmentation.
const SEGMENT_LENGTH: usize = (250 / CHUNK_LENGTH) * CHUNK_LENGTH;

pub struct Config {
    /// Response timeout in (ms)
    pub response_timeout: Duration,

    /// Transaction retry count
    pub transaction_retry: usize,

    /// Delay between chunks when transmitting (ms).
    ///
    /// See note on `segment_delay`.
    ///
    /// > `note-c`: https://github.com/blues/note-c/blob/master/n_lib.h#L52
    /// > Original: 20 ms
    pub chunk_delay: Duration,

    /// Delay between segments when transmitting (ms).
    ///
    /// > These delay may be almost eliminated for Notecard firmware version 3.4 (and presumably
    /// > above).
    ///
    /// > `note-c`: https://github.com/blues/note-c/blob/master/n_lib.h#L46
    /// > Original: 250 ms.
    pub segment_delay: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            response_timeout: Duration::seconds(5),
            transaction_retry: 5,
            chunk_delay: Duration::milliseconds(20),
            segment_delay: Duration::milliseconds(250),
        }
    }
}

pub struct SuspendState {
    config: Config,
    reset_required: bool,
}

pub struct Notecard<
    IFT: Read + Write,
    D: DelayNs,
    const BUF_SIZE: usize = DEFAULT_BUF_SIZE,
> {
    interface: IFT,
    delay: D,

    // Configuration
    config: Config,

    // State
    reset_required: bool,

    buffer: Vec<u8, BUF_SIZE>,
}

enum ResetResult {
    NonCRResult,
    CRResult,
}

impl<IFT:Read + Write, D: DelayNs> Notecard<IFT, D> {
    /// Create a new Notecard driver handler with the default configuration
    pub fn new(interface: IFT, delay: D) -> Self {
        Self::new_with_config(interface, delay, Config::default())
    }

    /// Create a new Notecard driver with a custom configuration
    pub fn new_with_config(interface: IFT, delay: D, config: Config) -> Self {
        Self {
            interface,
            delay,
            config,
            reset_required: true,
            buffer: Vec::new(),
        }
    }

    /// Release the IFT device by returning the interface and driver state and consuming the driver.
    pub fn suspend(self) -> (IFT, SuspendState) {
        (
            self.interface,
            SuspendState {
                config: self.config,
                reset_required: self.reset_required,
            },
        )
    }

    /// Recreate the driver from an existing state and an interface.
    pub fn resume(interface: IFT, delay: D, state: SuspendState) -> Self {
        Notecard {
            interface,
            delay,
            config: state.config,
            reset_required: state.reset_required,
            buffer: Vec::new(),
        }
    }

    /// Execute a json transaction
    pub async fn transaction<T: Serialize + NoteTransaction>(&mut self, cmd: T) -> Result<<T as NoteTransaction>::NoteResult, error::Error> {
        if self.reset_required {
            self.reset().await?;
            debug!("Reset Success!");
        }

        // Reset JSON buffer
        self.buffer.clear();
        self.buffer.resize(self.buffer.capacity(), 0).unwrap();

        // Serialize the command
        let size = serde_json_core::to_slice(&cmd, &mut self.buffer).map_err(|_| error::Error::SerError)?;
        self.buffer.truncate(size);

        // Add newline at the end of the JSON to indicate end of command to the notecard
        self.buffer.push(b'\n').map_err(|_| error::Error::SerError)?;

        // TODO: we need timout for the result read and a retry on the request
        self.send_request().await?;
        debug!("nc: Request sent...");

        self.read_result().await?;
        debug!("nc: received {:?}", core::str::from_utf8(&self.buffer).ok());

        Ok(cmd.parse(&self.buffer.as_slice())?)
    }

    /// Reset the Notecard
    pub async fn reset(&mut self) -> Result<(), error::Error> {
        debug!("Resetting communication interface");

        for _ in 0..self.config.transaction_retry {
            match self.try_reset().await {
                Ok(result) => match result {
                    ResetResult::NonCRResult => {
                        debug!("Found unexpected return characters. Retrying sync.")
                    }
                    ResetResult::CRResult => return Ok(()),
                },
                Err(err) => match err {
                    error::Error::TimeOut => (),
                    err => return Err(err),
                },
            }
        }

        Err(error::Error::TimeOut)
    }

    /// Attempt to reset the Notecard interface
    async fn try_reset(&mut self) -> Result<ResetResult, error::Error> {
        if let Err(e) = self.interface.write_all(b"\n").await {
            error!(
                "nc: Sending reset newline failed with {}",
                defmt::Debug2Format(&e)
            );
            self.delay
                .delay_ms(CARD_RESET_DRAIN_DELAY.num_milliseconds() as u32)
                .await;
        }

        let mut carrige_return_found = false;
        let mut newline_found = false;
        let mut not_control_char_found = false;
        loop {
            let mut buffer = [0_u8; 1];
            let read = select_biased! {
                char = self.interface.read(&mut buffer).fuse() => {
                    Some(char)
                },
                _ = self.delay.delay_ms(CARD_RESET_DRAIN_DELAY.num_milliseconds() as u32).fuse() => {
                    None
                }
            };

            match read {
                Some(res) => match res {
                    Ok(len) => {
                        trace!(
                            "nc: Got data count {} buffer {:?}",
                            len,
                            core::str::from_utf8(&buffer).unwrap()
                        )
                    }
                    Err(err) => {
                        error!("nc: Read failed with {}", defmt::Debug2Format(&err))
                    }
                },
                None => {
                    trace!("nc: Timeout");
                    break;
                }
            }

            match buffer[0] {
                b'\r' => carrige_return_found = true,
                b'\n' => newline_found = true,
                _ => not_control_char_found = true,
            }

            if carrige_return_found && newline_found {
                if not_control_char_found {
                    return Ok(ResetResult::NonCRResult);
                } else {
                    return Ok(ResetResult::CRResult);
                }
            }
        }

        Err(error::Error::TimeOut)
    }

    async fn send_request(&mut self) -> Result<(), error::Error> {
        if self.buffer.last() != Some(&b'\n') {
            return Err(error::Error::InvalidRequest);
        }

        trace!("nc: sending request: {:?}", core::str::from_utf8(&self.buffer).ok());

        for segment in self.buffer.chunks(SEGMENT_LENGTH) {
            self.interface.write_all(segment).await.map_err(|_| error::Error::WriteError)?;
            self.delay.delay_ms(self.config.segment_delay.num_milliseconds() as u32).await;
        }

        Ok(())
    }

    async fn read_result(&mut self) -> Result<(), error::Error> {
        // Clear the buffer
        self.buffer.clear();

        // Local variables
        let mut local_buffer: Vec<u8, 256> = Vec::new();
        let mut got_newline = false;
        let mut got_carriage = false;
        loop {
            local_buffer.clear();
            local_buffer.resize(local_buffer.capacity(), 0).ok();
            let available = self.interface.read(&mut local_buffer.as_mut_slice()).await.map_err(|_| error::Error::ReadError)?;
            if available == 0 {
                self.delay.delay_ms(10).await;
                continue;
            }
            local_buffer.truncate(available);
            trace!("nc: rr: len {} cont {:?}", local_buffer.len(), core::str::from_utf8(&local_buffer).ok());
            self.buffer.extend(local_buffer.iter().copied());
            if local_buffer.contains(&b'\n') {
                got_newline = true;
            }
            if local_buffer.contains(&b'\r') {
                got_carriage = true;
            }
            if got_newline && got_carriage {
                debug!("nc: rr: done!");
                break;
            }
        }

        Ok(())
    }
}

pub trait NoteTransaction {
    type NoteResult: DeserializeOwned;

    fn parse(&self, result: &[u8]) -> Result<Self::NoteResult, error::Error> {
        Ok(serde_json_core::from_slice::<Self::NoteResult>(&result).map_err(|_| error::Error::new_desererror(result))?.0)
    }
}
