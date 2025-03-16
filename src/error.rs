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