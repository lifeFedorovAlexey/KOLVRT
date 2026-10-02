//! Bounded immutable window reduction. One current contract: checked half-open spans.
//! CPU-owned borrowed data; no allocation, locking, IRQ access or retained state.
pub const MAX_WORDS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Bounds,
    Capacity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reduction {
    pub sum: u64,
    pub words: u32,
}
/// A caller-scoped native window capability. The provider owns data and authorization;
/// adapters supply only a checked half-open span, never another consumer's identity.
pub trait Backend {
    fn reduce(&mut self, span: Span) -> Result<Reduction, Error>;
}
pub struct Local<'a>(pub &'a [u32]);
impl Backend for Local<'_> {
    fn reduce(&mut self, span: Span) -> Result<Reduction, Error> {
        reduce(self.0, span)
    }
}
pub fn reduce(data: &[u32], span: Span) -> Result<Reduction, Error> {
    let words = checked(data, span)?;
    Ok(Reduction {
        sum: words.iter().map(|&v| u64::from(v)).sum(),
        words: words.len() as u32,
    })
}
/// Native bounds validation shared by a privileged read boundary and EL0 arithmetic.
pub fn checked(data: &[u32], span: Span) -> Result<&[u32], Error> {
    if data.len() > MAX_WORDS {
        return Err(Error::Capacity);
    }
    if span.start > span.end || span.end as usize > data.len() {
        return Err(Error::Bounds);
    }
    Ok(&data[span.start as usize..span.end as usize])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_contract() {
        assert_eq!(
            reduce(&[9, 2, 3], Span { start: 1, end: 3 }),
            Ok(Reduction { sum: 5, words: 2 })
        );
        assert_eq!(reduce(&[9], Span { start: 0, end: 0 }).unwrap().sum, 0);
        assert_eq!(reduce(&[9], Span { start: 1, end: 0 }), Err(Error::Bounds));
        assert_eq!(reduce(&[9], Span { start: 0, end: 2 }), Err(Error::Bounds));
        assert_eq!(
            reduce(&[0; MAX_WORDS + 1], Span { start: 0, end: 0 }),
            Err(Error::Capacity)
        );
    }
}
