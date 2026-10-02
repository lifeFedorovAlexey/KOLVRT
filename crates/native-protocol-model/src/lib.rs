//! Candidate request encoding for the first native slice, not a stable kernel ABI.
#![no_std]
#![forbid(unsafe_code)]

pub const HEADER: usize = 40;
pub const MAX_PAYLOAD: usize = 256;
pub const SEND: u32 = 1;
pub const TRANSFER: u32 = 2;
pub const KNOWN_RIGHTS: u32 = SEND | TRANSFER;
pub const RESPONSE_HEADER: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum Status {
    Completed = 0,
    Invalid = 1,
    Denied = 2,
    Exhausted = 3,
    Cancelled = 4,
    EffectUnknown = 5,
    Expired = 6,
    Unsupported = 7,
}

pub fn encode_response(
    id: u64,
    status: Status,
    payload: &[u8],
    output: &mut [u8],
) -> Result<usize, Error> {
    let size = RESPONSE_HEADER + payload.len();
    if payload.len() > MAX_PAYLOAD
        || output.len() < size
        || (status != Status::Completed && !payload.is_empty())
    {
        return Err(Error::Length);
    }
    output[..size].fill(0);
    output[2..4].copy_from_slice(&(status as u16).to_le_bytes());
    output[4..8].copy_from_slice(&(size as u32).to_le_bytes());
    output[8..16].copy_from_slice(&id.to_le_bytes());
    output[16..20].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    output[RESPONSE_HEADER..size].copy_from_slice(payload);
    Ok(size)
}

pub fn decode_response(bytes: &[u8], expected_id: u64) -> Result<(Status, &[u8]), Error> {
    if bytes.len() < RESPONSE_HEADER || bytes.len() > RESPONSE_HEADER + MAX_PAYLOAD {
        return Err(Error::Length);
    }
    if bytes[0..2] != [0, 0] {
        return Err(Error::Version);
    }
    let status = match u16::from_le_bytes(bytes[2..4].try_into().unwrap()) {
        0 => Status::Completed,
        1 => Status::Invalid,
        2 => Status::Denied,
        3 => Status::Exhausted,
        4 => Status::Cancelled,
        5 => Status::EffectUnknown,
        6 => Status::Expired,
        7 => Status::Unsupported,
        _ => return Err(Error::Operation),
    };
    let total = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let id = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let length = u32::from_le_bytes(bytes[16..20].try_into().unwrap()) as usize;
    if total != bytes.len()
        || length != bytes.len() - RESPONSE_HEADER
        || (status != Status::Completed && length != 0)
    {
        return Err(Error::Length);
    }
    if id != expected_id {
        return Err(Error::Stale);
    }
    if bytes[20..24] != [0; 4] {
        return Err(Error::Reserved);
    }
    Ok((status, &bytes[RESPONSE_HEADER..]))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Length,
    Version,
    Operation,
    Reserved,
    Deadline,
    Rights,
    Stale,
    Exhausted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub handle: u64,
    pub request_id: u64,
    pub deadline: u64,
    length: usize,
    payload: [u8; MAX_PAYLOAD],
}
impl Request {
    pub fn payload(&self) -> &[u8] {
        &self.payload[..self.length]
    }
}

/// Decode an already fault-safely copied snapshot. This function is not user-copy.
/// Deadline is an absolute tick in the negotiated boot-local monotonic clock.
pub fn decode(bytes: &[u8], now: u64) -> Result<Request, Error> {
    if bytes.len() < HEADER || bytes.len() > HEADER + MAX_PAYLOAD {
        return Err(Error::Length);
    }
    let u16_at = |i| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    let u32_at = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
    let u64_at = |i| u64::from_le_bytes(bytes[i..i + 8].try_into().unwrap());
    if u16_at(0) != 0 {
        return Err(Error::Version);
    }
    if u16_at(2) != 1 {
        return Err(Error::Operation);
    }
    if u32_at(4) as usize != bytes.len() || u32_at(32) as usize != bytes.len() - HEADER {
        return Err(Error::Length);
    }
    if u32_at(36) != 0 {
        return Err(Error::Reserved);
    }
    let deadline = u64_at(24);
    if deadline <= now {
        return Err(Error::Deadline);
    }
    let mut payload = [0; MAX_PAYLOAD];
    payload[..bytes.len() - HEADER].copy_from_slice(&bytes[HEADER..]);
    Ok(Request {
        handle: u64_at(8),
        request_id: u64_at(16),
        deadline,
        length: bytes.len() - HEADER,
        payload,
    })
}

pub fn delegate(held: u32, requested: u32) -> Result<u32, Error> {
    if held & !KNOWN_RIGHTS != 0
        || requested & !KNOWN_RIGHTS != 0
        || held & TRANSFER == 0
        || requested & !held != 0
    {
        return Err(Error::Rights);
    }
    Ok(requested)
}

/// Generation exhaustion permanently retires a slot rather than wrapping it.
pub fn next_generation(generation: u32) -> Result<u32, Error> {
    generation.checked_add(1).ok_or(Error::Exhausted)
}
pub fn handle(slot: u32, generation: u32) -> u64 {
    (u64::from(generation) << 32) | u64::from(slot)
}
pub fn resolve(token: u64, slot: u32, generation: u32, live: bool) -> Result<(), Error> {
    if !live || token != handle(slot, generation) {
        Err(Error::Stale)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn response_identity_and_poisoned_storage() {
        let mut bytes = [0xa5; RESPONSE_HEADER + MAX_PAYLOAD];
        let n = encode_response(9, Status::Completed, b"ok", &mut bytes).unwrap();
        assert_eq!(
            decode_response(&bytes[..n], 9),
            Ok((Status::Completed, &b"ok"[..]))
        );
        assert_eq!(decode_response(&bytes[..n], 10), Err(Error::Stale));
        for end in 0..n {
            assert!(decode_response(&bytes[..end], 9).is_err());
        }
        for offset in [0, 1, 2, 3, 4, 5, 6, 7, 16, 17, 18, 19, 20, 21, 22, 23] {
            let mut broken = bytes;
            broken[offset] ^= 0x80;
            assert!(decode_response(&broken[..n], 9).is_err());
        }
        assert!(encode_response(9, Status::EffectUnknown, b"false success", &mut bytes).is_err());
        for status in [
            Status::Invalid,
            Status::Denied,
            Status::Exhausted,
            Status::Cancelled,
            Status::EffectUnknown,
            Status::Expired,
            Status::Unsupported,
        ] {
            let n = encode_response(9, status, &[], &mut bytes).unwrap();
            assert_eq!(decode_response(&bytes[..n], 9), Ok((status, &[][..])));
        }
    }
    fn message() -> [u8; 44] {
        let mut b = [0; 44];
        b[2] = 1;
        b[4] = 44;
        b[24] = 10;
        b[32] = 4;
        b[40..].copy_from_slice(b"ping");
        b
    }
    #[test]
    fn owns_validated_snapshot() {
        let mut b = message();
        let request = decode(&b, 0).unwrap();
        b[40] = 0;
        assert_eq!(b[40], 0);
        assert_eq!(request.payload(), b"ping");
    }
    #[test]
    fn rejects_every_truncation_and_invalid_header() {
        let b = message();
        for end in 0..b.len() {
            assert!(decode(&b[..end], 0).is_err());
        }
        for offset in [0, 1, 2, 3, 4, 5, 6, 7, 32, 33, 34, 35, 36, 37, 38, 39] {
            let mut broken = b;
            broken[offset] ^= 0x80;
            assert!(decode(&broken, 0).is_err(), "offset {offset}");
        }
        assert_eq!(decode(&b, 10), Err(Error::Deadline));
        assert_eq!(
            decode(&[0; HEADER + MAX_PAYLOAD + 1], 0),
            Err(Error::Length)
        );
    }
    #[test]
    fn attenuation_and_generation_exhaustion() {
        for held in 0..16 {
            for requested in 0..16 {
                if let Ok(grant) = delegate(held, requested) {
                    assert_eq!(grant & !held, 0);
                    assert_eq!(held & TRANSFER, TRANSFER);
                }
            }
        }
        assert_eq!(delegate(SEND | TRANSFER, SEND), Ok(SEND));
        assert!(delegate(SEND, SEND).is_err());
        assert_eq!(next_generation(u32::MAX), Err(Error::Exhausted));
        assert!(resolve(handle(0, 1), 0, 2, true).is_err());
        assert!(resolve(handle(0, 1), 0, 1, false).is_err());
        assert_eq!(resolve(handle(0, 1), 0, 1, true), Ok(()));
    }
}
