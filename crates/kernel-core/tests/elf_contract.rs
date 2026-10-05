//! LAW-018 / bounded static ELF profile: mutate one field of an accepted image.
//! Host parser evidence only; this does not claim mapping or EL0 execution.
use kernel_core::elf::{self, LoadSegment, MAX_LOAD_SEGMENTS, PF_R, PF_W, PF_X};

const BASE: usize = 0x2004_0000;
const PAGE: usize = 4096;
const WINDOW: usize = 128 * 1024;

fn put(bytes: &mut [u8], offset: usize, value: u64, width: usize) {
    bytes[offset..offset + width].copy_from_slice(&value.to_le_bytes()[..width]);
}

fn image(loads: usize) -> Vec<u8> {
    let mut bytes = vec![0; (loads + 1) * PAGE];
    bytes[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
    for (offset, value, width) in [
        (16, 2, 2),
        (18, 183, 2),
        (20, 1, 4),
        (24, BASE as u64, 8),
        (32, 64, 8),
        (52, 64, 2),
        (54, 56, 2),
        (56, loads as u64, 2),
    ] {
        put(&mut bytes, offset, value, width);
    }
    for index in 0..loads {
        let at = 64 + index * 56;
        let flags = if index == 0 { PF_R | PF_X } else { PF_R | PF_W };
        for (offset, value, width) in [
            (0, 1, 4),
            (4, flags as u64, 4),
            (8, ((index + 1) * PAGE) as u64, 8),
            (16, (BASE + index * PAGE) as u64, 8),
            (32, 16, 8),
            (40, PAGE as u64, 8),
            (48, PAGE as u64, 8),
        ] {
            put(&mut bytes, at + offset, value, width);
        }
        bytes[(index + 1) * PAGE..(index + 1) * PAGE + 16].fill(0x31 + index as u8);
    }
    bytes
}

#[test]
fn accepted_image_preserves_every_load_field_payload_and_page_rounding() {
    let mut bytes = image(2);
    // Adjacent pages are valid; an incomplete final page still consumes a page.
    put(&mut bytes, 64 + 56 + 40, (PAGE - 1) as u64, 8);
    let plan = elf::parse(&bytes, BASE, WINDOW, PAGE).unwrap();
    assert_eq!(plan.bytes, bytes);
    assert_eq!(plan.entry, BASE);
    assert_eq!(plan.segment_count, 2);
    assert_eq!(plan.page_count, 2);
    assert_eq!(
        plan.segments[0],
        LoadSegment {
            file_offset: PAGE,
            file_size: 16,
            memory_size: PAGE,
            virtual_address: BASE,
            flags: PF_R | PF_X,
        }
    );
    assert_eq!(
        plan.segments[1],
        LoadSegment {
            file_offset: 2 * PAGE,
            file_size: 16,
            memory_size: PAGE - 1,
            virtual_address: BASE + PAGE,
            flags: PF_R | PF_W,
        }
    );
    assert!(
        plan.segments[2..]
            .iter()
            .all(|s| *s == LoadSegment::default())
    );
    for (index, segment) in plan.segments[..2].iter().enumerate() {
        assert_eq!(
            &plan.bytes[segment.file_offset..segment.file_offset + segment.file_size],
            &[0x31 + index as u8; 16]
        );
    }
    // The last aligned entry is valid, even with a large BSS tail.
    put(&mut bytes, 24, (BASE + 12) as u64, 8);
    assert_eq!(
        elf::parse(&bytes, BASE, WINDOW, PAGE).unwrap().entry,
        BASE + 12
    );
}

#[test]
fn entry_requires_aarch64_instruction_alignment() {
    for offset in [1, 2, 3, 15] {
        let mut bad = image(1);
        put(&mut bad, 24, (BASE + offset) as u64, 8);
        assert_eq!(
            elf::parse(&bad, BASE, WINDOW, PAGE).unwrap_err(),
            "AArch64 ELF entry is not instruction-aligned",
            "entry offset {offset}"
        );
    }
}

#[test]
fn aligned_entry_checks_file_geometry_without_decoding_an_instruction() {
    // Preserve the format contract: only the entry byte must be file-backed.
    // Full initial instruction backing/validity is not an execution guarantee.
    for file_size in [1, 2, 3] {
        let mut bytes = image(1);
        put(&mut bytes, 64 + 32, file_size, 8);
        assert_eq!(elf::parse(&bytes, BASE, WINDOW, PAGE).unwrap().entry, BASE);
    }
}

#[test]
fn identity_and_header_fields_fail_independently() {
    let good = image(1);
    elf::parse(&good, BASE, WINDOW, PAGE).unwrap();
    for (offset, value, width) in [
        (0, 0, 1),
        (4, 1, 1),
        (5, 2, 1),
        (6, 0, 1),
        (16, 3, 2),
        (18, 62, 2),
        (20, 0, 4),
        (52, 63, 2),
        (54, 55, 2),
    ] {
        let mut bad = good.clone();
        put(&mut bad, offset, value, width);
        assert!(
            elf::parse(&bad, BASE, WINDOW, PAGE).is_err(),
            "accepted header field {offset}"
        );
    }
}

#[test]
fn program_header_range_count_and_segment_limit_are_enforced() {
    let good = image(MAX_LOAD_SEGMENTS);
    assert_eq!(
        elf::parse(&good, BASE, WINDOW, PAGE).unwrap().segment_count,
        MAX_LOAD_SEGMENTS
    );
    let too_many = image(MAX_LOAD_SEGMENTS + 1);
    assert_eq!(
        elf::parse(&too_many, BASE, WINDOW, PAGE).unwrap_err(),
        "too many load segments"
    );
    for (offset, value, width, expected) in [
        (56, 0, 2, "program-header count exceeds profile limit"),
        (
            56,
            (MAX_LOAD_SEGMENTS + 5) as u64,
            2,
            "program-header count exceeds profile limit",
        ),
        (32, 63, 8, "program headers outside immutable image"),
        (
            32,
            good.len() as u64,
            8,
            "program headers outside immutable image",
        ),
        (32, u64::MAX, 8, "program-header range overflow"),
    ] {
        let mut bad = good.clone();
        put(&mut bad, offset, value, width);
        assert_eq!(
            elf::parse(&bad, BASE, WINDOW, PAGE).unwrap_err(),
            expected,
            "field {offset}={value}"
        );
    }
}

#[test]
fn segment_sizes_alignment_permissions_and_checked_arithmetic_fail_independently() {
    let good = image(1);
    elf::parse(&good, BASE, WINDOW, PAGE).unwrap();
    for (offset, value, width, expected) in [
        (40, 0, 8, "invalid segment size, range or permissions"),
        (
            32,
            (PAGE + 1) as u64,
            8,
            "invalid segment size, range or permissions",
        ),
        (
            8,
            (2 * PAGE) as u64,
            8,
            "invalid segment size, range or permissions",
        ),
        (
            4,
            PF_X as u64,
            4,
            "invalid segment size, range or permissions",
        ),
        (
            4,
            (PF_R | PF_W | PF_X) as u64,
            4,
            "invalid segment size, range or permissions",
        ),
        (
            4,
            (PF_R | PF_X | 8) as u64,
            4,
            "invalid segment size, range or permissions",
        ),
        (
            8,
            (PAGE + 1) as u64,
            8,
            "segment is misaligned or outside bounded user window",
        ),
        (
            16,
            (BASE + 1) as u64,
            8,
            "segment is misaligned or outside bounded user window",
        ),
        (
            48,
            1,
            8,
            "segment is misaligned or outside bounded user window",
        ),
        (
            16,
            (BASE - PAGE) as u64,
            8,
            "segment is misaligned or outside bounded user window",
        ),
        (
            40,
            (WINDOW + 1) as u64,
            8,
            "segment is misaligned or outside bounded user window",
        ),
        (8, u64::MAX, 8, "segment file range overflow"),
        (40, u64::MAX, 8, "segment address range overflow"),
    ] {
        let mut bad = good.clone();
        put(&mut bad, 64 + offset, value, width);
        assert_eq!(
            elf::parse(&bad, BASE, WINDOW, PAGE).unwrap_err(),
            expected,
            "segment field {offset}={value}"
        );
    }
}

#[test]
fn entry_requires_executable_file_bytes_and_overlap_is_order_independent() {
    let good = image(2);
    elf::parse(&good, BASE, WINDOW, PAGE).unwrap();
    for entry in [BASE - 4, BASE + 16, BASE + PAGE, BASE + WINDOW] {
        let mut bad = good.clone();
        put(&mut bad, 24, entry as u64, 8);
        assert_eq!(
            elf::parse(&bad, BASE, WINDOW, PAGE).unwrap_err(),
            "entry is not inside a file-backed executable segment"
        );
    }
    let mut reversed = good.clone();
    for byte in 0..56 {
        reversed.swap(64 + byte, 120 + byte);
    }
    let plan = elf::parse(&reversed, BASE, WINDOW, PAGE).unwrap();
    assert_eq!(plan.segments[0].virtual_address, BASE + PAGE);
    assert_eq!(plan.segments[1].virtual_address, BASE);
    for mut bad in [good, reversed] {
        put(&mut bad, 64 + 16, BASE as u64, 8);
        put(&mut bad, 120 + 16, BASE as u64, 8);
        assert_eq!(
            elf::parse(&bad, BASE, WINDOW, PAGE).unwrap_err(),
            "load segment pages overlap"
        );
    }
}

#[test]
fn unsupported_program_kinds_and_executable_stack_are_rejected() {
    let good = image(1);
    for kind in [2, 3, 7, 0x6474_e551] {
        let mut bad = good.clone();
        // Keep the executable load intact, so ignoring bad metadata would pass.
        put(&mut bad, 56, 2, 2);
        put(&mut bad, 120, kind, 4);
        put(&mut bad, 124, (PF_R | PF_X) as u64, 4);
        assert_eq!(
            elf::parse(&bad, BASE, WINDOW, PAGE).unwrap_err(),
            if kind == 0x6474_e551 {
                "executable stack is unsupported"
            } else {
                "dynamic, TLS and other program headers are unsupported"
            },
            "program kind {kind}"
        );
    }
    // Allowed metadata cannot silently discard a legitimate load segment.
    for kind in [0, 0x6474_e551] {
        let mut bytes = good.clone();
        put(&mut bytes, 56, 2, 2);
        put(&mut bytes, 120, kind, 4);
        put(&mut bytes, 124, (PF_R | PF_W) as u64, 4);
        assert_eq!(
            elf::parse(&bytes, BASE, WINDOW, PAGE)
                .unwrap()
                .segment_count,
            1
        );
        put(&mut bytes, 64, 0, 4);
        assert_eq!(
            elf::parse(&bytes, BASE, WINDOW, PAGE).unwrap_err(),
            "entry is not inside a file-backed executable segment"
        );
    }
}

#[test]
fn caller_window_accepts_exact_boundary_and_rejects_invalid_geometry() {
    let good = image(1);
    assert_eq!(elf::parse(&good, BASE, PAGE, PAGE).unwrap().page_count, 1);
    assert!(elf::parse(&good, BASE, PAGE - 1, PAGE).is_err());
    for (base, size, page, expected) in [
        (BASE, WINDOW, 0, "invalid user image bounds"),
        (BASE, WINDOW, 3, "invalid user image bounds"),
        (BASE, 0, PAGE, "invalid user image bounds"),
        (usize::MAX, 1, PAGE, "user image bounds overflow"),
    ] {
        assert_eq!(elf::parse(&good, base, size, page).unwrap_err(), expected);
    }
}
