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
    assert_eq!(d.cpu_count, platform_config::ACTIVE_CPUS);
    assert!(d.psci_smc);
    assert_ne!(d.cpu_affinities[0], d.cpu_affinities[1]);
    for len in 0..DTB.len() {
        assert!(discover(&DTB[..len]).is_err(), "accepted truncation {len}");
    }
}
#[test]
fn invalid_psci_conduit_and_cpu_enable_method() {
    for (old, replacement) in [
        (b"smc\0".as_slice(), b"hvc\0".as_slice()),
        (b"psci\0".as_slice(), b"spin\0".as_slice()),
    ] {
        let mut bad = DTB.to_vec();
        let offsets: Vec<_> = bad
            .windows(old.len())
            .enumerate()
            .filter_map(|(at, bytes)| (bytes == old).then_some(at))
            .collect();
        assert!(!offsets.is_empty());
        for at in offsets {
            bad[at..at + old.len()].copy_from_slice(replacement);
        }
        assert!(discover(&bad).is_err());
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

// Edit DTB input bytes, never production source or the implementation under test.
fn property_range(blob: &[u8], node: &str, property: &str) -> Option<(usize, usize)> {
    let word = |at| u32::from_be_bytes(blob[at..at + 4].try_into().unwrap()) as usize;
    let strings = word(FDT_STRINGS_OFFSET);
    let mut at = word(FDT_STRUCTURE_OFFSET);
    let mut stack = Vec::new();
    loop {
        let token_at = at;
        let token = word(at);
        at += 4;
        match token {
            1 => {
                let end = at + blob[at..].iter().position(|b| *b == 0).unwrap();
                stack.push(std::str::from_utf8(&blob[at..end]).unwrap());
                at = (end + 4) & !3;
            }
            2 => {
                stack.pop();
            }
            3 => {
                let length = word(at);
                let name = strings + word(at + 4);
                let end = name + blob[name..].iter().position(|b| *b == 0).unwrap();
                at = (at + 8 + length + 3) & !3;
                if stack.last() == Some(&node) && &blob[name..end] == property.as_bytes() {
                    return Some((token_at, at));
                }
            }
            4 => (),
            9 => return None,
            _ => panic!("invalid fixture token"),
        }
    }
}

fn replace_property(blob: &mut Vec<u8>, node: &str, name: &str, value: Option<&[u8]>) {
    let (start, end) = property_range(blob, node, name).expect("fixture property");
    let mut replacement = Vec::new();
    if let Some(value) = value {
        replacement.extend_from_slice(&3_u32.to_be_bytes());
        replacement.extend_from_slice(&(value.len() as u32).to_be_bytes());
        replacement.extend_from_slice(&blob[start + 8..start + 12]);
        replacement.extend_from_slice(value);
        replacement.resize((replacement.len() + 3) & !3, 0);
    }
    let delta = replacement.len() as i64 - (end - start) as i64;
    blob.splice(start..end, replacement);
    for offset in [
        FDT_TOTAL_SIZE_OFFSET,
        FDT_STRUCTURE_SIZE_OFFSET,
        FDT_STRINGS_OFFSET,
    ] {
        let old = u32::from_be_bytes(blob[offset..offset + 4].try_into().unwrap());
        blob[offset..offset + 4]
            .copy_from_slice(&u32::try_from(i64::from(old) + delta).unwrap().to_be_bytes());
    }
}

fn cells(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect()
}

#[test]
fn uart_spi_controller_and_extent_are_discovered() {
    let description = discover(DTB).unwrap();
    assert_eq!(description.uart.base, 0x0900_0000);
    assert_eq!(description.uart.size, 0x1000);
    assert_eq!(description.uart_irq, 33);
    assert_eq!(description.interrupt_controller, 0x8003);
}

#[test]
fn uart_rejects_forged_irq_and_controller_bindings() {
    for invalid in [
        cells(&[]),
        cells(&[0, 1]),
        cells(&[0, 1, 4, 0]),
        cells(&[1, 1, 4]),
        cells(&[0, 988, 4]),
        cells(&[0, u32::MAX, 4]),
        cells(&[0, 1, 1]),
        cells(&[0, 1, 0x104]),
    ] {
        let mut blob = DTB.to_vec();
        replace_property(&mut blob, "pl011@9000000", "interrupts", Some(&invalid));
        assert!(discover(&blob).is_err(), "accepted IRQ {invalid:?}");
    }
    for (node, property, values) in [
        ("", "interrupt-parent", vec![0x8000]),
        ("", "interrupt-parent", vec![0]),
        ("", "interrupt-parent", vec![u32::MAX]),
        ("", "interrupt-parent", vec![0x8003, 0]),
        ("intc@8000000", "phandle", vec![0x8000]),
        ("intc@8000000", "#interrupt-cells", vec![2]),
        ("intc@8000000", "interrupt-controller", vec![0]),
    ] {
        let mut blob = DTB.to_vec();
        replace_property(&mut blob, node, property, Some(&cells(&values)));
        assert!(
            discover(&blob).is_err(),
            "accepted {node}/{property} {values:?}"
        );
    }
    for (node, name) in [
        ("", "interrupt-parent"),
        ("intc@8000000", "phandle"),
        ("intc@8000000", "#interrupt-cells"),
        ("intc@8000000", "interrupt-controller"),
        ("pl011@9000000", "interrupts"),
    ] {
        let mut blob = DTB.to_vec();
        replace_property(&mut blob, node, name, None);
        assert!(discover(&blob).is_err(), "accepted missing {node}/{name}");
    }
}

#[test]
fn uart_rejects_overflow_overlap_and_nonexact_register_extent() {
    for values in [
        vec![0, 0x0900_0000, 0, 0],
        vec![0, 0x0900_0000, 0, 0x2000],
        vec![0, 0x0900_0001, 0, 0x1000],
        vec![u32::MAX, 0xffff_f000, 0, 0x1000],
        vec![0, 0x0800_0000, 0, 0x1000],
        vec![0, 0x080a_0000, 0, 0x1000],
        vec![0, 0x4000_0000, 0, 0x1000],
        vec![0, 0x0900_0000, 0, 0x1000, 0, 0x0901_0000, 0, 0x1000],
        vec![0, 0x0900_0000],
    ] {
        let mut blob = DTB.to_vec();
        replace_property(&mut blob, "pl011@9000000", "reg", Some(&cells(&values)));
        assert!(discover(&blob).is_err(), "accepted register {values:?}");
    }
}

fn insert_property_from(
    blob: &mut Vec<u8>,
    source_node: &str,
    name: &str,
    target_node: &str,
    anchor: &str,
) {
    let (start, end) = property_range(blob, source_node, name).unwrap();
    let bytes = blob[start..end].to_vec();
    let (at, _) = property_range(blob, target_node, anchor).unwrap();
    blob.splice(at..at, bytes.iter().copied());
    for offset in [
        FDT_TOTAL_SIZE_OFFSET,
        FDT_STRUCTURE_SIZE_OFFSET,
        FDT_STRINGS_OFFSET,
    ] {
        let old = u32::from_be_bytes(blob[offset..offset + 4].try_into().unwrap());
        blob[offset..offset + 4].copy_from_slice(&(old + bytes.len() as u32).to_be_bytes());
    }
}

#[test]
fn explicit_uart_parent_overrides_root_but_must_match_controller() {
    let mut blob = DTB.to_vec();
    insert_property_from(&mut blob, "", "interrupt-parent", "pl011@9000000", "reg");
    replace_property(&mut blob, "", "interrupt-parent", Some(&cells(&[0x8000])));
    assert_eq!(discover(&blob).unwrap().uart_irq, 33);
    replace_property(
        &mut blob,
        "pl011@9000000",
        "interrupt-parent",
        Some(&cells(&[0x8000])),
    );
    assert!(discover(&blob).is_err());
}

#[test]
fn duplicate_interrupt_properties_and_phandles_fail_closed() {
    let mut blob = DTB.to_vec();
    insert_property_from(
        &mut blob,
        "pl011@9000000",
        "interrupts",
        "pl011@9000000",
        "reg",
    );
    assert!(discover(&blob).is_err());
    let mut blob = DTB.to_vec();
    insert_property_from(&mut blob, "intc@8000000", "phandle", "pl011@9000000", "reg");
    assert!(discover(&blob).is_err());
}
