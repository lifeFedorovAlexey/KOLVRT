//! Checked parser for the deliberately small static AArch64 ELF64 profile.
//! This validates format and geometry; it does not authorize execution.

pub const MAX_LOAD_SEGMENTS: usize = 8;
pub const ET_EXEC: u16 = 2;
pub const EM_AARCH64: u16 = 183;
pub const PF_X: u32 = 1;
pub const PF_W: u32 = 2;
pub const PF_R: u32 = 4;
const ELF_HEADER_BYTES: usize = 64;
const PROGRAM_HEADER_BYTES: usize = 56;
const PT_NULL: u32 = 0;
const PT_LOAD: u32 = 1;
const PT_GNU_STACK: u32 = 0x6474_e551;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LoadSegment {
    pub file_offset: usize,
    pub file_size: usize,
    pub memory_size: usize,
    pub virtual_address: usize,
    pub flags: u32,
}

impl LoadSegment {
    pub fn end(self) -> Option<usize> {
        self.virtual_address.checked_add(self.memory_size)
    }
    pub fn page_start(self, page_size: usize) -> usize {
        self.virtual_address & !(page_size - 1)
    }
    pub fn page_end(self, page_size: usize) -> Option<usize> {
        self.end()?
            .checked_add(page_size - 1)
            .map(|end| end & !(page_size - 1))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Image<'a> {
    pub bytes: &'a [u8],
    pub entry: usize,
    pub segments: [LoadSegment; MAX_LOAD_SEGMENTS],
    pub segment_count: usize,
    pub page_count: usize,
}

fn u16_at(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}
fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}
fn u64_at(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        bytes.get(offset..offset.checked_add(8)?)?.try_into().ok()?,
    ))
}

/// Accept only fixed-address, little-endian AArch64 ET_EXEC with page-aligned,
/// nonoverlapping PT_LOAD segments inside the caller-provided user image window.
pub fn parse(
    bytes: &[u8],
    user_base: usize,
    user_bytes: usize,
    page_size: usize,
) -> Result<Image<'_>, &'static str> {
    if bytes.len() < ELF_HEADER_BYTES
        || bytes.get(..4) != Some(b"\x7fELF")
        || bytes[4] != 2
        || bytes[5] != 1
        || bytes[6] != 1
    {
        return Err("invalid ELF64 little-endian identity or truncated header");
    }
    if u16_at(bytes, 16) != Some(ET_EXEC)
        || u16_at(bytes, 18) != Some(EM_AARCH64)
        || u32_at(bytes, 20) != Some(1)
        || u16_at(bytes, 52) != Some(ELF_HEADER_BYTES as u16)
        || u16_at(bytes, 54) != Some(PROGRAM_HEADER_BYTES as u16)
    {
        return Err("unsupported ELF type, machine, version or header geometry");
    }
    if page_size == 0 || !page_size.is_power_of_two() || user_bytes == 0 {
        return Err("invalid user image bounds");
    }
    let user_end = user_base
        .checked_add(user_bytes)
        .ok_or("user image bounds overflow")?;
    let entry = usize::try_from(u64_at(bytes, 24).ok_or("truncated ELF entry")?)
        .map_err(|_| "ELF entry does not fit address space")?;
    let phoff = usize::try_from(u64_at(bytes, 32).ok_or("truncated program-header offset")?)
        .map_err(|_| "program-header offset does not fit address space")?;
    let phnum = usize::from(u16_at(bytes, 56).ok_or("truncated program-header count")?);
    if phnum == 0 || phnum > MAX_LOAD_SEGMENTS + 4 {
        return Err("program-header count exceeds profile limit");
    }
    let phbytes = phnum
        .checked_mul(PROGRAM_HEADER_BYTES)
        .ok_or("program-header size overflow")?;
    let phend = phoff
        .checked_add(phbytes)
        .ok_or("program-header range overflow")?;
    if phoff < ELF_HEADER_BYTES || phend > bytes.len() {
        return Err("program headers outside immutable image");
    }

    let mut segments = [LoadSegment::default(); MAX_LOAD_SEGMENTS];
    let mut segment_count = 0usize;
    let mut page_count = 0usize;
    let mut executable_entry = false;
    for index in 0..phnum {
        let at = phoff + index * PROGRAM_HEADER_BYTES;
        let kind = u32_at(bytes, at).ok_or("truncated program header")?;
        let flags = u32_at(bytes, at + 4).ok_or("truncated program flags")?;
        if kind == PT_NULL {
            continue;
        }
        if kind == PT_GNU_STACK {
            if flags & PF_X != 0 {
                return Err("executable stack is unsupported");
            }
            continue;
        }
        if kind != PT_LOAD {
            return Err("dynamic, TLS and other program headers are unsupported");
        }
        if segment_count == MAX_LOAD_SEGMENTS {
            return Err("too many load segments");
        }
        let file_offset = usize::try_from(u64_at(bytes, at + 8).ok_or("truncated segment offset")?)
            .map_err(|_| "segment offset does not fit address space")?;
        let virtual_address =
            usize::try_from(u64_at(bytes, at + 16).ok_or("truncated segment address")?)
                .map_err(|_| "segment address does not fit address space")?;
        let file_size =
            usize::try_from(u64_at(bytes, at + 32).ok_or("truncated segment file size")?)
                .map_err(|_| "segment file size does not fit address space")?;
        let memory_size =
            usize::try_from(u64_at(bytes, at + 40).ok_or("truncated segment memory size")?)
                .map_err(|_| "segment memory size does not fit address space")?;
        let alignment =
            usize::try_from(u64_at(bytes, at + 48).ok_or("truncated segment alignment")?)
                .map_err(|_| "segment alignment does not fit address space")?;
        let file_end = file_offset
            .checked_add(file_size)
            .ok_or("segment file range overflow")?;
        let memory_end = virtual_address
            .checked_add(memory_size)
            .ok_or("segment address range overflow")?;
        if memory_size == 0
            || file_size > memory_size
            || file_end > bytes.len()
            || flags & PF_R == 0
            || flags & !(PF_R | PF_W | PF_X) != 0
            || flags & (PF_W | PF_X) == (PF_W | PF_X)
        {
            return Err("invalid segment size, range or permissions");
        }
        if !virtual_address.is_multiple_of(page_size)
            || !file_offset.is_multiple_of(page_size)
            || alignment != page_size
            || virtual_address < user_base
            || memory_end > user_end
        {
            return Err("segment is misaligned or outside bounded user window");
        }
        let page_end = memory_end
            .checked_add(page_size - 1)
            .ok_or("segment page range overflow")?
            & !(page_size - 1);
        let segment_pages = (page_end - virtual_address) / page_size;
        page_count = page_count
            .checked_add(segment_pages)
            .ok_or("image page count overflow")?;
        let current = LoadSegment {
            file_offset,
            file_size,
            memory_size,
            virtual_address,
            flags,
        };
        for prior in &segments[..segment_count] {
            let prior_end = prior
                .page_end(page_size)
                .ok_or("prior segment range overflow")?;
            if virtual_address < prior_end && prior.page_start(page_size) < page_end {
                return Err("load segment pages overlap");
            }
        }
        if flags & PF_X != 0 && entry >= virtual_address && entry < virtual_address + file_size {
            executable_entry = true;
        }
        segments[segment_count] = current;
        segment_count += 1;
    }
    if segment_count == 0 || !executable_entry {
        return Err("entry is not inside a file-backed executable segment");
    }
    Ok(Image {
        bytes,
        entry,
        segments,
        segment_count,
        page_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;
    use std::vec::Vec;
    const BASE: usize = 0x2004_0000;
    const PAGE: usize = 4096;

    fn fixture(
        flags: u32,
        entry: usize,
        address: usize,
        file_offset: usize,
        filesz: usize,
        memsz: usize,
    ) -> Vec<u8> {
        let mut bytes = vec![0u8; file_offset + filesz.clamp(1, 16)];
        bytes[..16].copy_from_slice(b"\x7fELF\x02\x01\x01\0\0\0\0\0\0\0\0\0");
        put16(&mut bytes, 16, ET_EXEC);
        put16(&mut bytes, 18, EM_AARCH64);
        put32(&mut bytes, 20, 1);
        put64(&mut bytes, 24, entry as u64);
        put64(&mut bytes, 32, 64);
        put16(&mut bytes, 52, 64);
        put16(&mut bytes, 54, 56);
        put16(&mut bytes, 56, 1);
        put32(&mut bytes, 64, PT_LOAD);
        put32(&mut bytes, 68, flags);
        put64(&mut bytes, 72, file_offset as u64);
        put64(&mut bytes, 80, address as u64);
        put64(&mut bytes, 96, filesz as u64);
        put64(&mut bytes, 104, memsz as u64);
        put64(&mut bytes, 112, PAGE as u64);
        bytes
    }
    fn put16(b: &mut [u8], o: usize, v: u16) {
        b[o..o + 2].copy_from_slice(&v.to_le_bytes());
    }
    fn put32(b: &mut [u8], o: usize, v: u32) {
        b[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn put64(b: &mut [u8], o: usize, v: u64) {
        b[o..o + 8].copy_from_slice(&v.to_le_bytes());
    }

    #[test]
    fn accepts_one_bounded_executable_segment_and_zero_fill_geometry() {
        let elf = fixture(PF_R | PF_X, BASE, BASE, PAGE, 12, PAGE);
        let plan = parse(&elf, BASE, 128 * 1024, PAGE).unwrap();
        assert_eq!(plan.entry, BASE);
        assert_eq!(plan.page_count, 1);
        assert_eq!(plan.segments[0].memory_size, PAGE);
    }
    #[test]
    fn truncation_overflow_overlap_and_permissions_fail_closed() {
        let good = fixture(PF_R | PF_X, BASE, BASE, PAGE, 12, PAGE);
        for end in 0..good.len() {
            assert!(parse(&good[..end], BASE, 128 * 1024, PAGE).is_err());
        }
        assert!(
            parse(
                &fixture(PF_R | PF_W | PF_X, BASE, BASE, PAGE, 4, PAGE),
                BASE,
                128 * 1024,
                PAGE
            )
            .is_err()
        );
        assert!(
            parse(
                &fixture(PF_R | PF_X, BASE, usize::MAX & !(PAGE - 1), PAGE, 4, PAGE),
                BASE,
                128 * 1024,
                PAGE
            )
            .is_err()
        );
        assert!(
            parse(
                &fixture(PF_R | PF_X, BASE, BASE, PAGE, usize::MAX, PAGE),
                BASE,
                128 * 1024,
                PAGE
            )
            .is_err()
        );
        let mut overlap = fixture(PF_R | PF_X, BASE, BASE, PAGE, 4, PAGE);
        overlap.resize(PAGE * 2 + 4, 0);
        put16(&mut overlap, 56, 2);
        let second = 64 + 56;
        put32(&mut overlap, second, PT_LOAD);
        put32(&mut overlap, second + 4, PF_R | PF_X);
        put64(&mut overlap, second + 8, (PAGE * 2) as u64);
        put64(&mut overlap, second + 16, BASE as u64);
        put64(&mut overlap, second + 32, 4);
        put64(&mut overlap, second + 40, PAGE as u64);
        put64(&mut overlap, second + 48, PAGE as u64);
        assert!(parse(&overlap, BASE, 128 * 1024, PAGE).is_err());
    }
    #[test]
    fn rejects_foreign_virtual_address_and_bad_entry() {
        assert!(
            parse(
                &fixture(PF_R | PF_X, BASE, BASE - PAGE, PAGE, 4, PAGE),
                BASE,
                128 * 1024,
                PAGE
            )
            .is_err()
        );
        assert!(
            parse(
                &fixture(PF_R | PF_X, BASE + PAGE, BASE, PAGE, 4, PAGE),
                BASE,
                128 * 1024,
                PAGE
            )
            .is_err()
        );
    }
}
