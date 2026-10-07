//! Pure validation of immutable bootstrap grant shape, shared by the kernel Scope.
//! This validates fixed authority bounds, not policy, provenance or image format.
use crate::domain::Limits;
#[derive(Clone, Copy)]
pub struct Grant<'a, Format> {
    pub image: &'a [u8],
    pub image_format: Format,
    pub send_to: Option<usize>,
    pub owner: usize,
    pub limits: Limits,
    pub instances: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    EmptyImage,
    InstanceCredits,
    SendEdge,
}
pub fn validate_grants<Format>(grants: &[Grant<'_, Format>]) -> Result<(), Error> {
    for (index, grant) in grants.iter().enumerate() {
        if grant.image.is_empty() {
            return Err(Error::EmptyImage);
        }
        if grant.instances == 0 || grant.instances > 8 {
            return Err(Error::InstanceCredits);
        }
        if grant
            .send_to
            .is_some_and(|target| target >= grants.len() || target == index)
        {
            return Err(Error::SendEdge);
        }
    }
    Ok(())
}
