//! Native byte-copy limits and range validation; no pointer dereference or ABI.
pub const MAX_BYTES: usize = 3 * 4096;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    TooLarge,
    InvalidRange,
    PermissionDenied,
    UserFault { copied: usize },
    StaleContext,
}
/// Empty ranges still require a non-null address inside the user aperture.
/// Byte copies have no alignment requirement. Limit precedes address checks.
pub fn range(address: usize, length: usize, start: usize, end: usize) -> Result<(), Error> {
    if length > MAX_BYTES {
        return Err(Error::TooLarge);
    }
    let stop = address.checked_add(length).ok_or(Error::InvalidRange)?;
    if address == 0 || address < start || address >= end || stop > end {
        return Err(Error::InvalidRange);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_byte_ranges() {
        let base = 0x2000_0000;
        let end = base + 0x20_0000;
        for size in [0, 1, 4096, MAX_BYTES] {
            assert_eq!(range(base + 1, size, base, end), Ok(()));
        }
        assert_eq!(range(base, MAX_BYTES + 1, base, end), Err(Error::TooLarge));
        for (address, length) in [
            (0, 0),
            (0, 1),
            (end, 0),
            (end - 1, 2),
            (usize::MAX, 2),
            (0x4020_0000, 1),
        ] {
            assert_eq!(range(address, length, base, end), Err(Error::InvalidRange));
        }
    }
}
