#![no_std]
#![forbid(unsafe_code)]
//! Synthetic demonstration protocols, not a Linux ABI. Legacy types stay here.
use kernel_core::window::Reduction;
#[cfg(any(feature = "v1", feature = "v2", feature = "bug"))]
use kernel_core::window::{Backend, Local, Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Encoding,
    Bounds,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    pub translations: u64,
    pub copied_bytes: u64,
    pub conversions: u64,
    pub backend_calls: u64,
}
#[cfg(feature = "v1")]
const INCLUSIVE_FIELD_BYTES: usize = core::mem::size_of::<u16>();
#[cfg(any(feature = "v1", feature = "v2", feature = "bug"))]
const PROTOCOL_FIELDS: usize = 2; // Both versions encode exactly two endpoints/start-count fields.
#[cfg(feature = "v1")]
const INCLUSIVE_FRAME_BYTES: usize = PROTOCOL_FIELDS * INCLUSIVE_FIELD_BYTES;
#[cfg(any(feature = "v2", feature = "bug"))]
const COUNTED_FIELD_BYTES: usize = core::mem::size_of::<u32>();
#[cfg(any(feature = "v2", feature = "bug"))]
const COUNTED_FRAME_BYTES: usize = PROTOCOL_FIELDS * COUNTED_FIELD_BYTES;
pub type Outcome = (Result<Reduction, Error>, Work);
// Fixed owned snapshots prevent validation/execution observing different bytes.
#[cfg(any(feature = "v1", feature = "v2", feature = "bug"))]
fn snapshot<const N: usize>(bytes: &[u8]) -> Result<[u8; N], Error> {
    bytes.try_into().map_err(|_| Error::Encoding)
}
#[cfg(any(feature = "v1", feature = "v2", feature = "bug"))]
fn backend(provider: &mut impl Backend, span: Span) -> Result<Reduction, Error> {
    provider.reduce(span).map_err(|_| Error::Bounds)
}
#[cfg(feature = "v1")]
pub fn v1(data: &[u32], bytes: &[u8]) -> Outcome {
    v1_with(&mut Local(data), bytes)
}
#[cfg(feature = "v1")]
pub fn v1_with(provider: &mut impl Backend, bytes: &[u8]) -> Outcome {
    let mut work = Work::default();
    let result = (|| {
        // v1: inclusive LE16 endpoints; all-ones pair is the empty sentinel.
        let frame = snapshot::<INCLUSIVE_FRAME_BYTES>(bytes)?;
        work.translations = 1;
        work.copied_bytes = INCLUSIVE_FRAME_BYTES as u64;
        work.conversions = PROTOCOL_FIELDS as u64;
        let first = u16::from_le_bytes(frame[..INCLUSIVE_FIELD_BYTES].try_into().unwrap());
        let last = u16::from_le_bytes(frame[INCLUSIVE_FIELD_BYTES..].try_into().unwrap());
        let span = if first == u16::MAX && last == u16::MAX {
            Span { start: 0, end: 0 }
        } else {
            if first > last {
                return Err(Error::Bounds);
            }
            work.conversions += 1;
            Span {
                start: u32::from(first),
                end: u32::from(last) + 1,
            }
        };
        work.backend_calls = 1;
        backend(provider, span)
    })();
    (result, work)
}
#[cfg(any(feature = "v2", feature = "bug"))]
fn counted(provider: &mut impl Backend, bytes: &[u8], empty_first: bool) -> Outcome {
    let mut work = Work::default();
    let result = (|| {
        // v2: BE32 start/count, checked sum. Historical bug treats zero count as one.
        let frame = snapshot::<COUNTED_FRAME_BYTES>(bytes)?;
        work.translations = 1;
        work.copied_bytes = COUNTED_FRAME_BYTES as u64;
        work.conversions = PROTOCOL_FIELDS as u64 + u64::from(empty_first);
        let first = u32::from_be_bytes(frame[..COUNTED_FIELD_BYTES].try_into().unwrap());
        let count = u32::from_be_bytes(frame[COUNTED_FIELD_BYTES..].try_into().unwrap());
        let count = if empty_first && count == 0 { 1 } else { count };
        let end = first.checked_add(count).ok_or(Error::Bounds)?;
        work.conversions += 1;
        work.backend_calls = 1;
        backend(provider, Span { start: first, end })
    })();
    (result, work)
}
#[cfg(feature = "v2")]
pub fn v2(data: &[u32], bytes: &[u8]) -> Outcome {
    v2_with(&mut Local(data), bytes)
}
#[cfg(feature = "v2")]
pub fn v2_with(provider: &mut impl Backend, bytes: &[u8]) -> Outcome {
    counted(provider, bytes, false)
}
#[cfg(feature = "bug")]
pub fn empty_first(data: &[u32], bytes: &[u8]) -> Outcome {
    empty_first_with(&mut Local(data), bytes)
}
#[cfg(feature = "bug")]
pub fn empty_first_with(provider: &mut impl Backend, bytes: &[u8]) -> Outcome {
    counted(provider, bytes, true)
}
