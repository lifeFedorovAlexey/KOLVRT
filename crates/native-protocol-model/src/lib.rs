//! Candidate request encoding for the first native slice, not a stable kernel ABI.
#![no_std]
#![forbid(unsafe_code)]

pub const HEADER: usize = 40;
pub const MAX_PAYLOAD: usize = 256;
pub const SEND: u32 = 1;
pub const TRANSFER: u32 = 2;
pub const KNOWN_RIGHTS: u32 = SEND | TRANSFER;
pub const RESPONSE_HEADER: usize = 24;
pub const PROTOCOL_VERSION: u16 = 0;
pub const OPERATION_SEND: u16 = 1;

/// Byte ranges of the candidate little-endian wire format.
pub mod request_fields {
    use core::ops::Range;
    pub const VERSION: Range<usize> = 0..2;
    pub const OPERATION: Range<usize> = 2..4;
    pub const TOTAL_SIZE: Range<usize> = 4..8;
    pub const HANDLE: Range<usize> = 8..16;
    pub const REQUEST_ID: Range<usize> = 16..24;
    pub const DEADLINE: Range<usize> = 24..32;
    pub const PAYLOAD_SIZE: Range<usize> = 32..36;
    pub const RESERVED: Range<usize> = 36..40;
}
pub mod response_fields {
    use core::ops::Range;
    pub const VERSION: Range<usize> = 0..2;
    pub const STATUS: Range<usize> = 2..4;
    pub const TOTAL_SIZE: Range<usize> = 4..8;
    pub const REQUEST_ID: Range<usize> = 8..16;
    pub const PAYLOAD_SIZE: Range<usize> = 16..20;
    pub const RESERVED: Range<usize> = 20..24;
}

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
    output[response_fields::STATUS].copy_from_slice(&(status as u16).to_le_bytes());
    output[response_fields::TOTAL_SIZE].copy_from_slice(&(size as u32).to_le_bytes());
    output[response_fields::REQUEST_ID].copy_from_slice(&id.to_le_bytes());
    output[response_fields::PAYLOAD_SIZE].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    output[RESPONSE_HEADER..size].copy_from_slice(payload);
    Ok(size)
}

pub fn decode_response(bytes: &[u8], expected_id: u64) -> Result<(Status, &[u8]), Error> {
    if bytes.len() < RESPONSE_HEADER || bytes.len() > RESPONSE_HEADER + MAX_PAYLOAD {
        return Err(Error::Length);
    }
    if bytes[response_fields::VERSION] != PROTOCOL_VERSION.to_le_bytes() {
        return Err(Error::Version);
    }
    let status = match u16::from_le_bytes(bytes[response_fields::STATUS].try_into().unwrap()) {
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
    let total = u32::from_le_bytes(bytes[response_fields::TOTAL_SIZE].try_into().unwrap()) as usize;
    let id = u64::from_le_bytes(bytes[response_fields::REQUEST_ID].try_into().unwrap());
    let length =
        u32::from_le_bytes(bytes[response_fields::PAYLOAD_SIZE].try_into().unwrap()) as usize;
    if total != bytes.len()
        || length != bytes.len() - RESPONSE_HEADER
        || (status != Status::Completed && length != 0)
    {
        return Err(Error::Length);
    }
    if id != expected_id {
        return Err(Error::Stale);
    }
    if bytes[response_fields::RESERVED] != [0; core::mem::size_of::<u32>()] {
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
    let u16_at =
        |field: core::ops::Range<usize>| u16::from_le_bytes(bytes[field].try_into().unwrap());
    let u32_at =
        |field: core::ops::Range<usize>| u32::from_le_bytes(bytes[field].try_into().unwrap());
    let u64_at =
        |field: core::ops::Range<usize>| u64::from_le_bytes(bytes[field].try_into().unwrap());
    if u16_at(request_fields::VERSION) != PROTOCOL_VERSION {
        return Err(Error::Version);
    }
    if u16_at(request_fields::OPERATION) != OPERATION_SEND {
        return Err(Error::Operation);
    }
    if u32_at(request_fields::TOTAL_SIZE) as usize != bytes.len()
        || u32_at(request_fields::PAYLOAD_SIZE) as usize != bytes.len() - HEADER
    {
        return Err(Error::Length);
    }
    if u32_at(request_fields::RESERVED) != 0 {
        return Err(Error::Reserved);
    }
    let deadline = u64_at(request_fields::DEADLINE);
    if deadline <= now {
        return Err(Error::Deadline);
    }
    let mut payload = [0; MAX_PAYLOAD];
    payload[..bytes.len() - HEADER].copy_from_slice(&bytes[HEADER..]);
    Ok(Request {
        handle: u64_at(request_fields::HANDLE),
        request_id: u64_at(request_fields::REQUEST_ID),
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
    (u64::from(generation) << u32::BITS) | u64::from(slot)
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
    const TEST_PAYLOAD: &[u8; 4] = b"ping";
    const TEST_DEADLINE: u64 = 10;
    const TEST_REQUEST_ID: u64 = 9;
    const STORAGE_POISON: u8 = 0xa5;
    const CORRUPTION_BIT: u8 = 0x80;
    #[test]
    fn response_identity_and_poisoned_storage() {
        let mut bytes = [STORAGE_POISON; RESPONSE_HEADER + MAX_PAYLOAD];
        let n = encode_response(TEST_REQUEST_ID, Status::Completed, b"ok", &mut bytes).unwrap();
        assert_eq!(
            decode_response(&bytes[..n], TEST_REQUEST_ID),
            Ok((Status::Completed, &b"ok"[..]))
        );
        assert_eq!(
            decode_response(&bytes[..n], TEST_REQUEST_ID + 1),
            Err(Error::Stale)
        );
        for end in 0..n {
            assert!(decode_response(&bytes[..end], TEST_REQUEST_ID).is_err());
        }
        for offset in (response_fields::VERSION.start..response_fields::TOTAL_SIZE.end)
            .chain(response_fields::PAYLOAD_SIZE.start..response_fields::RESERVED.end)
        {
            let mut broken = bytes;
            broken[offset] ^= CORRUPTION_BIT;
            assert!(decode_response(&broken[..n], TEST_REQUEST_ID).is_err());
        }
        assert!(
            encode_response(
                TEST_REQUEST_ID,
                Status::EffectUnknown,
                b"false success",
                &mut bytes
            )
            .is_err()
        );
        for status in [
            Status::Invalid,
            Status::Denied,
            Status::Exhausted,
            Status::Cancelled,
            Status::EffectUnknown,
            Status::Expired,
            Status::Unsupported,
        ] {
            let n = encode_response(TEST_REQUEST_ID, status, &[], &mut bytes).unwrap();
            assert_eq!(
                decode_response(&bytes[..n], TEST_REQUEST_ID),
                Ok((status, &[][..]))
            );
        }
    }
    fn message() -> [u8; HEADER + TEST_PAYLOAD.len()] {
        let mut b = [0; HEADER + TEST_PAYLOAD.len()];
        let size = b.len() as u32;
        b[request_fields::OPERATION].copy_from_slice(&OPERATION_SEND.to_le_bytes());
        b[request_fields::TOTAL_SIZE].copy_from_slice(&size.to_le_bytes());
        b[request_fields::DEADLINE].copy_from_slice(&TEST_DEADLINE.to_le_bytes());
        b[request_fields::PAYLOAD_SIZE].copy_from_slice(&(TEST_PAYLOAD.len() as u32).to_le_bytes());
        b[HEADER..].copy_from_slice(TEST_PAYLOAD);
        b
    }
    #[test]
    fn owns_validated_snapshot() {
        let mut b = message();
        let request = decode(&b, 0).unwrap();
        b[HEADER] = 0;
        assert_eq!(b[HEADER], 0);
        assert_eq!(request.payload(), b"ping");
    }
    #[test]
    fn unsupported_versions_never_decode_as_current_contract() {
        let mut request = message();
        let mut response = [0; RESPONSE_HEADER];
        encode_response(TEST_REQUEST_ID, Status::Completed, &[], &mut response).unwrap();
        for version in 1..=u16::MAX {
            request[request_fields::VERSION].copy_from_slice(&version.to_le_bytes());
            response[response_fields::VERSION].copy_from_slice(&version.to_le_bytes());
            assert_eq!(decode(&request, 0), Err(Error::Version));
            assert_eq!(
                decode_response(&response, TEST_REQUEST_ID),
                Err(Error::Version)
            );
        }
    }
    #[test]
    fn rejects_every_truncation_and_invalid_header() {
        let b = message();
        for end in 0..b.len() {
            assert!(decode(&b[..end], 0).is_err());
        }
        for offset in (request_fields::VERSION.start..request_fields::TOTAL_SIZE.end)
            .chain(request_fields::PAYLOAD_SIZE.start..request_fields::RESERVED.end)
        {
            let mut broken = b;
            broken[offset] ^= CORRUPTION_BIT;
            assert!(decode(&broken, 0).is_err(), "offset {offset}");
        }
        assert_eq!(decode(&b, TEST_DEADLINE), Err(Error::Deadline));
        assert_eq!(
            decode(&[0; HEADER + MAX_PAYLOAD + 1], 0),
            Err(Error::Length)
        );
    }
    #[test]
    fn attenuation_and_generation_exhaustion() {
        // Include two unknown bits as well as every known-rights combination.
        let rights_domain = 1 << (KNOWN_RIGHTS.count_ones() + 2);
        for held in 0..rights_domain {
            for requested in 0..rights_domain {
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
