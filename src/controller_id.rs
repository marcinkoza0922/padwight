//! A controller's stable id: what a saved setting (calibration, "don't manage") is keyed by, so
//! it follows the physical pad rather than its name, and two identical pads can differ.

/// The unique id the driver reports (serial, Bluetooth MAC) when there is one, else `vid:pid`.
/// Identical pads without a unique id share one id.
pub fn controller_id(vendor: u16, product: u16, uniq: Option<&str>) -> String {
    match uniq.map(str::trim) {
        Some(u) if !u.is_empty() => u.to_string(),
        _ => format!("{vendor:04x}:{product:04x}"),
    }
}

#[cfg(test)]
mod tests {
    use super::controller_id;

    #[test]
    fn unique_id_wins_over_vendor_and_product() {
        assert_eq!(controller_id(0x054c, 0x0ce6, Some("aa:bb:cc")), "aa:bb:cc");
    }

    #[test]
    fn falls_back_to_vendor_and_product() {
        assert_eq!(controller_id(0x054c, 0x0ce6, None), "054c:0ce6");
        assert_eq!(controller_id(0x045e, 0x028e, Some(" ")), "045e:028e");
    }
}
