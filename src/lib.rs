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

mod error;

const CARD_RESET_DRAIN_DELAY: Duration = Duration::milliseconds(500);

pub trait Now {
    // The time elapsed since startup in microseconds
    fn now_micros(&self) -> u64;
}

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

pub struct Notecard<IFT: Read + Write, C: Now, D: DelayNs> {
    interface: IFT,
    _clock: C,
    delay: D,

    // Configuration
    config: Config,

    // State
    reset_required: bool,
}

enum ResetResult {
    NonCRResult,
    CRResult,
}

impl<IFT: Read + Write, C: Now, D: DelayNs> Notecard<IFT, C, D> {
    /// Create a new Notecard driver handler with the default configuration
    pub fn new(interface: IFT, clock: C, delay: D) -> Self {
        Self::new_with_config(interface, clock, delay, Config::default())
    }

    /// Create a new Notecard driver with a custom configuration
    pub fn new_with_config(interface: IFT, clock: C, delay: D, config: Config) -> Self {
        Self {
            interface,
            _clock: clock,
            delay,
            config,
            reset_required: true,
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
    pub fn resume(interface: IFT, clock: C, delay: D, state: SuspendState) -> Self {
        Notecard {
            interface,
            _clock: clock,
            delay,
            config: state.config,
            reset_required: state.reset_required,
        }
    }

    /// Execute a json transaction
    pub async fn transaction(&mut self) -> Result<(), error::Error> {
        if self.reset_required {
            self.reset().await?;
            trace!("Reset Success!");
        }

        Ok(())
    }

    /// Reset the Notecard
    pub async fn reset(&mut self) -> Result<(), error::Error> {
        debug!("Resetting communication interface");

        for _ in 0..self.config.transaction_retry {
            match self.try_reset().await {
                Ok(result) => match result {
                    ResetResult::NonCRResult => {
                        trace!("Found unexpected return characters. Retrying sync.")
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
                "Sending reset newline failed with {}",
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
                            "Got data count {} buffer {:?}",
                            len,
                            core::str::from_utf8(&buffer).unwrap()
                        )
                    }
                    Err(err) => {
                        error!("Read failed with {}", defmt::Debug2Format(&err))
                    }
                },
                None => {
                    trace!("Timeout");
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
}
