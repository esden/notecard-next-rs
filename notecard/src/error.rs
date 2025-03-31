use heapless::String;

#[derive(Debug, defmt::Format, Clone)]
pub enum Error {
    WriteError,

    ReadError,

    DeserError(String<256>),

    SerError,

    /// Request does not end with '\n'.
    InvalidRequest,

    RemainingData,

    TimeOut,

    BufOverflow,

    /// Method called when notecarrier is in invalid state.
    WrongState,

    /// Notecard firmware is being updated.
    DFUInProgress,

    /// Notecard filesystem full
    FileStorageFull(String<256>),

    /// Error Adding Note
    ErrorAddingNote(String<256>),

    NotecardErr(String<256>),
}

impl Error {
    pub fn new_desererror(msg: &[u8]) -> Error {
        let msg = core::str::from_utf8(&msg).unwrap_or("[invalid utf8]");
        let mut s = String::new();
        s.push_str(msg).ok();
        Error::DeserError(s)
    }
}