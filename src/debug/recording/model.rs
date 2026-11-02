//! What a recording holds: the devices it was taken from and every event, with its time.

/// Event types and codes the recording code needs by number.
pub const EV_SYN: u16 = 0;
pub const EV_KEY: u16 = 1;
pub const EV_REL: u16 = 2;
pub const EV_ABS: u16 = 3;
pub const EV_MSC: u16 = 4;
#[cfg(test)]
pub const SYN_REPORT: u16 = 0;
pub const SYN_DROPPED: u16 = 3;

/// One absolute axis of a device: its code and the ranges the kernel reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AbsSpec {
    pub code: u16,
    pub min: i32,
    pub max: i32,
    pub fuzz: i32,
    pub flat: i32,
    pub res: i32,
}

/// An input device as it was recorded: its identity and everything it can report.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeviceSpec {
    pub name: String,
    pub bus: u16,
    pub vendor: u16,
    pub product: u16,
    pub version: u16,
    /// The unique ID (a Bluetooth address, for one); empty when the device has none.
    pub uniq: String,
    pub phys: String,
    pub props: Vec<u16>,
    pub keys: Vec<u16>,
    pub rel: Vec<u16>,
    pub abs: Vec<AbsSpec>,
    pub msc: Vec<u16>,
    pub sw: Vec<u16>,
    pub led: Vec<u16>,
    pub ff: Vec<u16>,
}

/// One input event: when (microseconds since the recording began), on which device (an index
/// into [`Recording::devices`]), and the kernel's type, code and value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub micros: u64,
    pub device: usize,
    pub kind: u16,
    pub code: u16,
    pub value: i32,
}

impl Event {
    pub fn is_sync(&self) -> bool {
        self.kind == EV_SYN
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recording {
    pub devices: Vec<DeviceSpec>,
    pub events: Vec<Event>,
}
