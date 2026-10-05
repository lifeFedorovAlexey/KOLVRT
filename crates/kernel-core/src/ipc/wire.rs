//! native.request/1: immutable copied bytes, explicit LE fields, no Rust ABI.
use super::{Error, Outcome, Payload, ServiceToken};
pub const VERSION: u16 = 1;
pub const HEADER: usize = 40;
pub const MAX_FRAME: usize = HEADER + super::MAX_PAYLOAD;
pub const RESULT_HEADER: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Submit,
    Receive,
    Commit,
    Reply,
    Wait,
    Collect,
    Cancel,
    Shutdown,
    Abandon,
}
impl Operation {
    pub fn decode(value: u16) -> Result<Self, Error> {
        match value {
            1 => Ok(Self::Submit),
            2 => Ok(Self::Receive),
            3 => Ok(Self::Commit),
            4 => Ok(Self::Reply),
            5 => Ok(Self::Wait),
            6 => Ok(Self::Collect),
            7 => Ok(Self::Cancel),
            8 => Ok(Self::Shutdown),
            9 => Ok(Self::Abandon),
            _ => Err(Error::Unsupported),
        }
    }
    pub fn encode(self) -> u16 {
        match self {
            Self::Submit => 1,
            Self::Receive => 2,
            Self::Commit => 3,
            Self::Reply => 4,
            Self::Wait => 5,
            Self::Collect => 6,
            Self::Cancel => 7,
            Self::Shutdown => 8,
            Self::Abandon => 9,
        }
    }
}
pub struct Input {
    pub operation: Operation,
    /// Caller-local endpoint handle, or caller-local accepted receipt token.
    pub selector: u64,
    /// Submit client ID or exact service request token; never a caller principal.
    pub id: u64,
    pub deadline: u64,
    pub payload: Payload,
}
fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}
impl Input {
    pub fn decode(snapshot: &[u8]) -> Result<Self, Error> {
        if snapshot.len() < HEADER || snapshot.len() > MAX_FRAME {
            return Err(Error::Invalid);
        }
        if u16_at(snapshot, 0) != VERSION {
            return Err(Error::Unsupported);
        }
        let operation = Operation::decode(u16_at(snapshot, 2))?;
        let length = usize::try_from(u32_at(snapshot, 32)).map_err(|_| Error::Invalid)?;
        if HEADER.checked_add(length) != Some(snapshot.len())
            || usize::try_from(u32_at(snapshot, 4)).ok() != Some(snapshot.len())
            || u32_at(snapshot, 36) != 0
        {
            return Err(Error::Invalid);
        }
        let selector = u64_at(snapshot, 8);
        let id = u64_at(snapshot, 16);
        let deadline = u64_at(snapshot, 24);
        if operation != Operation::Submit && deadline != 0 {
            return Err(Error::Invalid);
        }
        if !matches!(operation, Operation::Submit | Operation::Reply) && length != 0 {
            return Err(Error::Invalid);
        }
        if !matches!(
            operation,
            Operation::Submit | Operation::Reply | Operation::Commit
        ) && id != 0
        {
            return Err(Error::Invalid);
        }
        Ok(Self {
            operation,
            selector,
            id,
            deadline,
            payload: Payload::copy(&snapshot[HEADER..])?,
        })
    }
}

/// A fully initialized owned frame; only its exact initialized prefix is observable.
pub struct Output {
    bytes: [u8; MAX_FRAME],
    length: usize,
}
impl Output {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..self.length]
    }
    pub fn received(token: ServiceToken, client_id: u64, deadline: u64, payload: &Payload) -> Self {
        let mut output = Self {
            bytes: [0; MAX_FRAME],
            length: HEADER + payload.len(),
        };
        output.bytes[..2].copy_from_slice(&VERSION.to_le_bytes());
        output.bytes[4..8].copy_from_slice(&(output.length as u32).to_le_bytes());
        output.bytes[8..16].copy_from_slice(&token.encode().to_le_bytes());
        output.bytes[16..24].copy_from_slice(&client_id.to_le_bytes());
        output.bytes[24..32].copy_from_slice(&deadline.to_le_bytes());
        output.bytes[32..36].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        output.bytes[HEADER..output.length].copy_from_slice(payload.bytes());
        output
    }
    pub fn result(client_id: u64, outcome: Outcome, payload: &Payload) -> Self {
        assert!(outcome == Outcome::Completed || payload.is_empty());
        let mut output = Self {
            bytes: [0; MAX_FRAME],
            length: RESULT_HEADER + payload.len(),
        };
        output.bytes[..2].copy_from_slice(&VERSION.to_le_bytes());
        output.bytes[2..4].copy_from_slice(&outcome.encode().to_le_bytes());
        output.bytes[4..8].copy_from_slice(&(output.length as u32).to_le_bytes());
        output.bytes[8..16].copy_from_slice(&client_id.to_le_bytes());
        output.bytes[16..20].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        output.bytes[RESULT_HEADER..output.length].copy_from_slice(payload.bytes());
        output
    }
}
