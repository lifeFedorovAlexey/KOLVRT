use kernel_core::platform::{
    FDT_HEADER_BYTES, FDT_RESERVATIONS_OFFSET, FDT_STRINGS_OFFSET, FDT_STRINGS_SIZE_OFFSET,
    FDT_STRUCTURE_OFFSET, FDT_STRUCTURE_SIZE_OFFSET, FDT_TOTAL_SIZE_OFFSET, FDT_VERSION,
    FDT_VERSION_OFFSET, discover,
};
#[path = "../../kernel/src/platform/config.rs"]
#[allow(dead_code)]
mod platform_config;
const DTB: &[u8] = include_bytes!("../../../research/fixtures/virt-10.1.dtb");
#[test]
fn actual_qemu_description_and_every_truncation() {
    let d = discover(DTB).unwrap();
    assert_eq!(d.ram.base, platform_config::RAM_BASE as u64);
    assert_eq!(d.timer_irq, platform_config::PHYSICAL_TIMER_IRQ);
    for len in 0..DTB.len() {
        assert!(discover(&DTB[..len]).is_err(), "accepted truncation {len}");
    }
}
#[test]
fn malformed_extents_alignment_and_version() {
    for (offset, value) in [
        (FDT_TOTAL_SIZE_OFFSET, u32::MAX),
        (FDT_STRUCTURE_OFFSET, FDT_HEADER_BYTES as u32 + 1),
        (FDT_STRINGS_OFFSET, u32::MAX),
        (FDT_RESERVATIONS_OFFSET, FDT_HEADER_BYTES as u32 + 1),
        (FDT_VERSION_OFFSET, FDT_VERSION - 1),
        (FDT_STRINGS_SIZE_OFFSET, u32::MAX),
        (FDT_STRUCTURE_SIZE_OFFSET, u32::MAX),
    ] {
        let mut bad = DTB.to_vec();
        bad[offset..offset + core::mem::size_of::<u32>()].copy_from_slice(&value.to_be_bytes());
        assert!(discover(&bad).is_err());
    }
    let mut bad = DTB.to_vec();
    bad[0] = 0;
    assert!(discover(&bad).is_err());
}
