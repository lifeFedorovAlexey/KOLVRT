#![no_std]
#![forbid(unsafe_code)]
use kernel_core::window::{Backend, Local, Reduction, Span};
use sha2::{Digest, Sha256};
include!(concat!(env!("OUT_DIR"), "/identity.rs"));
#[cfg(any(test, feature = "conformance"))]
pub mod conformance;

pub const CONSUMERS: usize = 4;
pub const PROFILE_SCHEMA: u16 = 1;
pub const NATIVE_CONTRACT: u16 = 1;
const DIGEST_BYTES: usize = 32; // SHA-256 width.
const SCHEMA_OFFSET: usize = 4; // Follows the four-byte KVRT magic.
const NATIVE_VERSION_OFFSET: usize = SCHEMA_OFFSET + core::mem::size_of::<u16>();
const GENERATION_OFFSET: usize = NATIVE_VERSION_OFFSET + core::mem::size_of::<u16>();
const CONSUMER_COUNT_OFFSET: usize = GENERATION_OFFSET + core::mem::size_of::<u32>();
const HEADER: usize = CONSUMER_COUNT_OFFSET + 1;
const ENTRY_ROUTE_OFFSET: usize = 1;
const ENTRY_IDENTITY_OFFSET: usize = ENTRY_ROUTE_OFFSET + 1;
const ENTRY: usize = ENTRY_IDENTITY_OFFSET + DIGEST_BYTES;
const ROUTE_COUNT: usize = 4;
pub const PROFILE_BYTES: usize = HEADER + CONSUMERS * ENTRY + DIGEST_BYTES;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    Native = 0,
    Inclusive = 1,
    Counted = 2,
    EmptyFirst = 3,
}
impl Route {
    pub const ALL: [Self; ROUTE_COUNT] = [
        Self::Native,
        Self::Inclusive,
        Self::Counted,
        Self::EmptyFirst,
    ];
    pub fn parse(id: u8) -> Result<Self, Error> {
        Self::ALL
            .get(id as usize)
            .copied()
            .ok_or(Error::Unsupported)
    }
    pub const fn available(self) -> bool {
        match self {
            Self::Native => true,
            Self::Inclusive => cfg!(feature = "compat-v1"),
            Self::Counted => cfg!(feature = "compat-v2"),
            Self::EmptyFirst => cfg!(feature = "bug-compat"),
        }
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::Native => "window.native/1.0.0",
            Self::Inclusive => "window.inclusive/1.0.0",
            Self::Counted => "window.counted/2.0.0",
            Self::EmptyFirst => "bug.window.empty-first/1.0.0",
        }
    }
    pub const fn reason(self) -> &'static str {
        match self {
            Self::Native => "current half-open span",
            Self::Inclusive => "inclusive LE16 endpoints and empty sentinel",
            Self::Counted => "BE32 start/count records",
            Self::EmptyFirst => "synthetic old consumer expects one word for zero count",
        }
    }
    pub fn identity(self) -> [u8; DIGEST_BYTES] {
        let mut hash = Sha256::new();
        hash.update(self.name().as_bytes());
        hash.update(NATIVE_ID);
        if self != Self::Native {
            hash.update(ADAPTER_ID);
        }
        hash.finalize().into()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unsupported,
    Encoding,
    Bounds,
    Busy,
    Generation,
    ProfileVersion,
    Integrity,
    Identity,
    Consumer,
}
#[derive(Clone, Copy)]
pub enum Input<'a> {
    Native(Span),
    Encoded(&'a [u8]),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    pub translations: u64,
    pub copies: u64,
    pub copied_bytes: u64,
    pub conversions: u64,
    pub backend_calls: u64,
}

/// Only DEV retains counters. Shared backend calls are separate from external admissions.
#[cfg(feature = "dev")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    pub native_calls: u64,
    pub compat_calls: u64,
    pub backend_calls: u64,
    pub translations: u64,
    pub copies: u64,
    pub copied_bytes: u64,
    pub conversions: u64,
    pub errors: u64,
    pub ticks: u64,
    pub fallbacks: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Unknown,
    Legacy,
    Compat,
    Mixed,
    MostlyNative,
    Native,
}
impl Status {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::Legacy => "LEGACY",
            Self::Compat => "COMPAT",
            Self::Mixed => "MIXED",
            Self::MostlyNative => "MOSTLY_NATIVE",
            Self::Native => "NATIVE",
        }
    }
    pub const fn color(self) -> &'static str {
        match self {
            Self::Unknown => "none",
            Self::Legacy => "brown",
            Self::Compat => "red",
            Self::Mixed => "yellow",
            Self::MostlyNative => "light-green",
            Self::Native => "green",
        }
    }
}
// Consumer-specific migration budget: at most one translated admission per 32 calls.
// This demo budget targets a startup record followed by 31 native operations; no universal score.
#[cfg(feature = "dev")]
pub fn classify(
    declared_native: bool,
    declared_compat: bool,
    c: Counters,
    budget: Option<u64>,
) -> Status {
    let calls = c.native_calls + c.compat_calls;
    if calls == 0 {
        return Status::Unknown;
    }
    if !declared_compat && c.compat_calls == 0 {
        return Status::Native;
    }
    if !declared_native {
        return Status::Legacy;
    }
    if c.native_calls == 0 {
        return Status::Compat;
    }
    if budget.is_some_and(|minimum| minimum > 0 && c.compat_calls.saturating_mul(minimum) <= calls)
    {
        Status::MostlyNative
    } else {
        Status::Mixed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profile {
    generation: u32,
    routes: [Route; CONSUMERS],
}
impl Profile {
    pub fn new(generation: u32, routes: [Route; CONSUMERS]) -> Result<Self, Error> {
        if generation == 0 {
            return Err(Error::Generation);
        }
        if routes.iter().any(|r| !r.available()) {
            return Err(Error::Unsupported);
        }
        Ok(Self { generation, routes })
    }
    pub const fn native() -> Self {
        Self {
            generation: 1,
            routes: [Route::Native; CONSUMERS],
        }
    }
    pub const fn routes(&self) -> &[Route; CONSUMERS] {
        &self.routes
    }
    pub const fn generation(&self) -> u32 {
        self.generation
    }
    pub fn encode(&self) -> [u8; PROFILE_BYTES] {
        let mut bytes = [0; PROFILE_BYTES];
        bytes[..SCHEMA_OFFSET].copy_from_slice(b"KVRT");
        bytes[SCHEMA_OFFSET..NATIVE_VERSION_OFFSET].copy_from_slice(&PROFILE_SCHEMA.to_le_bytes());
        bytes[NATIVE_VERSION_OFFSET..GENERATION_OFFSET]
            .copy_from_slice(&NATIVE_CONTRACT.to_le_bytes());
        bytes[GENERATION_OFFSET..CONSUMER_COUNT_OFFSET]
            .copy_from_slice(&self.generation.to_le_bytes());
        bytes[CONSUMER_COUNT_OFFSET] = CONSUMERS as u8;
        for (id, route) in self.routes.iter().enumerate() {
            let at = HEADER + id * ENTRY;
            bytes[at] = id as u8;
            bytes[at + ENTRY_ROUTE_OFFSET] = *route as u8;
            bytes[at + ENTRY_IDENTITY_OFFSET..at + ENTRY].copy_from_slice(&route.identity());
        }
        let digest: [u8; DIGEST_BYTES] =
            Sha256::digest(&bytes[..PROFILE_BYTES - DIGEST_BYTES]).into();
        bytes[PROFILE_BYTES - DIGEST_BYTES..].copy_from_slice(&digest);
        bytes
    }
    /// Caller must additionally authenticate this blob by a trusted expected digest.
    /// SHA-256 integrity is not a signature and does not establish operator authority.
    pub fn decode(bytes: &[u8], expected: [u8; DIGEST_BYTES]) -> Result<Self, Error> {
        if bytes.len() != PROFILE_BYTES {
            return Err(Error::Encoding);
        }
        let digest: [u8; DIGEST_BYTES] =
            Sha256::digest(&bytes[..PROFILE_BYTES - DIGEST_BYTES]).into();
        if bytes[PROFILE_BYTES - DIGEST_BYTES..] != digest || expected != digest {
            return Err(Error::Integrity);
        }
        if &bytes[..SCHEMA_OFFSET] != b"KVRT"
            || bytes[SCHEMA_OFFSET..NATIVE_VERSION_OFFSET] != PROFILE_SCHEMA.to_le_bytes()
            || bytes[NATIVE_VERSION_OFFSET..GENERATION_OFFSET] != NATIVE_CONTRACT.to_le_bytes()
        {
            return Err(Error::ProfileVersion);
        }
        if bytes[CONSUMER_COUNT_OFFSET] != CONSUMERS as u8 {
            return Err(Error::Consumer);
        }
        let mut routes = [Route::Native; CONSUMERS];
        for (id, route) in routes.iter_mut().enumerate() {
            let at = HEADER + id * ENTRY;
            if bytes[at] != id as u8 {
                return Err(Error::Consumer);
            }
            *route = Route::parse(bytes[at + ENTRY_ROUTE_OFFSET])?;
            if bytes[at + ENTRY_IDENTITY_OFFSET..at + ENTRY] != route.identity() {
                return Err(Error::Identity);
            }
        }
        Self::new(
            u32::from_le_bytes(
                bytes[GENERATION_OFFSET..CONSUMER_COUNT_OFFSET]
                    .try_into()
                    .unwrap(),
            ),
            routes,
        )
    }
}

/// One exclusively owned synchronous consumer.
#[cfg_attr(
    not(feature = "dev"),
    doc = "Production deliberately has no rebind API.\n```compile_fail\nlet mut c = routing::Consumer::from_profile(&routing::Profile::native(), 0).unwrap();\nc.switch(routing::Route::Native).unwrap();\n```"
)]
#[cfg_attr(feature = "dev", doc = "DEV adds transaction-boundary rebind methods.")]
pub struct Consumer {
    route: Route,
    generation: u32,
    busy: bool,
    #[cfg(feature = "dev")]
    counters: [Counters; ROUTE_COUNT],
}
impl Consumer {
    pub fn from_profile(profile: &Profile, id: usize) -> Result<Self, Error> {
        let route = *profile.routes.get(id).ok_or(Error::Consumer)?;
        Ok(Self {
            route,
            generation: profile.generation,
            busy: false,
            #[cfg(feature = "dev")]
            counters: [Counters::default(); ROUTE_COUNT],
        })
    }
    pub const fn route(&self) -> Route {
        self.route
    }
    pub const fn generation(&self) -> u32 {
        self.generation
    }
    #[cfg(feature = "dev")]
    pub fn counters(&self, route: Route) -> Counters {
        self.counters[route as usize]
    }
    #[cfg(feature = "dev")]
    pub fn switch(&mut self, route: Route) -> Result<(), Error> {
        if self.busy {
            return Err(Error::Busy);
        }
        if !route.available() {
            return Err(Error::Unsupported);
        }
        let generation = self.generation.checked_add(1).ok_or(Error::Generation)?;
        self.route = route;
        self.generation = generation;
        Ok(())
    }
    pub fn begin(&mut self) -> Result<Transaction<'_>, Error> {
        if self.busy {
            return Err(Error::Busy);
        }
        self.busy = true;
        Ok(Transaction { consumer: self })
    }
    pub fn call(&mut self, data: &[u32], input: Input<'_>) -> Result<(Reduction, Work), Error> {
        self.begin()?.call(data, input)
    }
    pub fn call_with(
        &mut self,
        provider: &mut impl Backend,
        input: Input<'_>,
    ) -> Result<(Reduction, Work), Error> {
        self.begin()?.call_with(provider, input)
    }
}
/// Exclusive borrow keeps consumer identity and binding pinned until synchronous work ends.
/// No heap or IRQ use. A Consumer has one exclusive owner; different consumers may run
/// concurrently on different CPUs. The provider cannot reborrow this Consumer.
/// Drop is the synchronous transaction boundary; no references escape a call.
pub struct Transaction<'a> {
    consumer: &'a mut Consumer,
}
impl Transaction<'_> {
    #[cfg(feature = "dev")]
    pub fn try_switch(&mut self, route: Route) -> Result<(), Error> {
        self.consumer.switch(route)
    }
    pub fn call(&mut self, data: &[u32], input: Input<'_>) -> Result<(Reduction, Work), Error> {
        self.call_with(&mut Local(data), input)
    }
    pub fn call_with(
        &mut self,
        provider: &mut impl Backend,
        input: Input<'_>,
    ) -> Result<(Reduction, Work), Error> {
        let route = self.consumer.route;
        #[cfg(feature = "dev")]
        {
            let c = &mut self.consumer.counters[route as usize];
            if route == Route::Native {
                c.native_calls += 1;
            } else {
                c.compat_calls += 1;
            }
        }
        let (result, work) = dispatch(route, provider, input);
        #[cfg(feature = "dev")]
        {
            let c = &mut self.consumer.counters[route as usize];
            c.backend_calls += work.backend_calls;
            c.translations += work.translations;
            c.copies += work.copies;
            c.copied_bytes += work.copied_bytes;
            c.conversions += work.conversions;
            if result.is_err() {
                c.errors += 1;
            }
        }
        result.map(|r| (r, work))
    }
    #[cfg(feature = "dev")]
    pub fn charge_ticks(&mut self, ticks: u64) {
        self.consumer.counters[self.consumer.route as usize].ticks += ticks;
    }
}
impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        self.consumer.busy = false;
    }
}

fn dispatch(
    route: Route,
    provider: &mut impl Backend,
    input: Input<'_>,
) -> (Result<Reduction, Error>, Work) {
    if route == Route::Native {
        return match input {
            Input::Native(span) => (
                provider.reduce(span).map_err(|_| Error::Bounds),
                Work {
                    backend_calls: 1,
                    ..Work::default()
                },
            ),
            _ => (Err(Error::Encoding), Work::default()),
        };
    }
    let Input::Encoded(bytes) = input else {
        return (Err(Error::Encoding), Work::default());
    };
    let _ = (&provider, bytes);
    match route {
        #[cfg(feature = "compat-v1")]
        Route::Inclusive => adapted(window_compat::v1_with(provider, bytes)),
        #[cfg(feature = "compat-v2")]
        Route::Counted => adapted(window_compat::v2_with(provider, bytes)),
        #[cfg(feature = "bug-compat")]
        Route::EmptyFirst => adapted(window_compat::empty_first_with(provider, bytes)),
        _ => (Err(Error::Unsupported), Work::default()),
    }
}
#[cfg(any(feature = "compat-v1", feature = "compat-v2", feature = "bug-compat"))]
fn adapted((result, w): window_compat::Outcome) -> (Result<Reduction, Error>, Work) {
    (
        result.map_err(|e| match e {
            window_compat::Error::Encoding => Error::Encoding,
            window_compat::Error::Bounds => Error::Bounds,
        }),
        Work {
            translations: w.translations,
            copies: u64::from(w.copied_bytes > 0),
            copied_bytes: w.copied_bytes,
            conversions: w.conversions,
            backend_calls: w.backend_calls,
        },
    )
}
