//! What a replica answers its driver with.

/// The device side of a replica: what it replies when the kernel driver asks for a report or
/// writes one.
pub trait Responder: Send + 'static {
    /// The reply to a request for report `number` (feature reports, mostly), report id first.
    /// `None` fails the request.
    fn get_report(&mut self, number: u8) -> Option<Vec<u8>>;

    /// An output report the driver wrote, report id first. Returns the input reports the device
    /// sends back in answer.
    fn output(&mut self, _data: &[u8]) -> Vec<Vec<u8>> {
        Vec::new()
    }
}
