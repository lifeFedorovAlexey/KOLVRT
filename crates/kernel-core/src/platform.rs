pub const FDT_HEADER_BYTES: usize = 40;
pub const FDT_TOTAL_SIZE_OFFSET: usize = 4;
pub const FDT_STRUCTURE_OFFSET: usize = 8;
pub const FDT_STRINGS_OFFSET: usize = 12;
pub const FDT_RESERVATIONS_OFFSET: usize = 16;
pub const FDT_VERSION_OFFSET: usize = 20;
pub const FDT_COMPATIBLE_VERSION_OFFSET: usize = 24;
pub const FDT_STRINGS_SIZE_OFFSET: usize = 32;
pub const FDT_STRUCTURE_SIZE_OFFSET: usize = 36;
pub const FDT_VERSION: u32 = 17;
pub const FDT_BEGIN_NODE: u32 = 1;
pub const FDT_END_NODE: u32 = 2;
pub const FDT_PROPERTY: u32 = 3;
pub const FDT_NOP: u32 = 4;
pub const FDT_END: u32 = 9;
const CELL_BYTES: usize = core::mem::size_of::<u32>();
const WIDE_BYTES: usize = core::mem::size_of::<u64>();
const REGION_BYTES: usize = 2 * WIDE_BYTES;
const PROPERTY_HEADER_BYTES: usize = 2 * CELL_BYTES;
const IRQ_SPECIFIER_BYTES: usize = 3 * CELL_BYTES;
const PHYSICAL_TIMER_SPECIFIER: usize = IRQ_SPECIFIER_BYTES;
const MAX_RESERVED_REGIONS: usize = 32;
const MAX_NODE_PROPERTIES: usize = 32;
const MAX_NODE_DEPTH: usize = 16;
const ADDRESS_SIZE_CELLS: u32 = 2;
const CPU_ADDRESS_CELLS_NARROW: u32 = 1;
const CPU_SIZE_CELLS: u32 = 0;
const CPU_NODE_DEPTH: usize = 2; // /cpus/cpu@... below the root.
pub const FDT_MAGIC: u32 = 0xd00dfeed;
const GIC_IRQ_TYPE_PPI: u32 = 1;
const GIC_IRQ_TRIGGER_MASK: u32 = 15;
const GIC_IRQ_LEVEL_HIGH: u32 = 4;
const GIC_PPI_BASE: u32 = 16;
const GIC_PPI_END: u32 = 32;
const PL011_REGISTER_SIZE: u64 = 0x1000;
const GICD_REGISTER_SIZE: u64 = 0x10000;
const GICR_FRAME_SIZE: u64 = 0x20000;
pub const MPIDR_AFFINITY_MASK: u64 = 0xff00ffffff;
pub const MAX_BOOT_CPUS: usize = 2;
#[derive(Clone, Copy, Debug, Default)]
pub struct Region {
    pub base: u64,
    pub size: u64,
}
impl Region {
    pub fn end(self) -> Result<u64, &'static str> {
        self.base.checked_add(self.size).ok_or("region overflow")
    }
}
#[derive(Debug)]
pub struct Description {
    pub ram: Region,
    pub uart: Region,
    pub distributor: Region,
    pub redistributor: Region,
    pub timer_irq: u32,
    pub reserved: [Region; MAX_RESERVED_REGIONS],
    pub reserved_count: usize,
    pub cpu_affinities: [u64; MAX_BOOT_CPUS],
    pub cpu_count: usize,
    pub psci_smc: bool,
}
fn word(b: &[u8], i: usize) -> Result<u32, &'static str> {
    Ok(u32::from_be_bytes(
        b.get(i..i.checked_add(CELL_BYTES).ok_or("offset")?)
            .ok_or("truncated word")?
            .try_into()
            .map_err(|_| "word")?,
    ))
}
fn wide(b: &[u8], i: usize) -> Result<u64, &'static str> {
    Ok((u64::from(word(b, i)?) << u32::BITS) | u64::from(word(b, i + CELL_BYTES)?))
}
fn cstr(b: &[u8]) -> Result<&[u8], &'static str> {
    b.get(
        ..b.iter()
            .position(|&v| v == 0)
            .ok_or("unterminated string")?,
    )
    .ok_or("string")
}
fn region(b: &[u8], offset: usize) -> Result<Region, &'static str> {
    let r = Region {
        base: wide(b, offset)?,
        size: wide(b, offset + WIDE_BYTES)?,
    };
    r.end()?;
    if r.size == 0 {
        return Err("empty region");
    }
    Ok(r)
}
#[derive(Clone, Copy, Default)]
struct Node<'a> {
    name: &'a [u8],
    compatible: &'a [u8],
    reg: &'a [u8],
    irq: &'a [u8],
    reserved: bool,
    properties: [&'a [u8]; MAX_NODE_PROPERTIES],
    property_count: usize,
    disabled: bool,
    enable_method: &'a [u8],
    method: &'a [u8],
    device_type: &'a [u8],
    address_cells: Option<u32>,
    size_cells: Option<u32>,
}

/// Bounded FDT v17 decoder for the pinned virt topology. Unknown devices are not enabled.
pub fn discover(input: &[u8]) -> Result<Description, &'static str> {
    if word(input, 0)? != FDT_MAGIC
        || word(input, FDT_VERSION_OFFSET)? != FDT_VERSION
        || word(input, FDT_COMPATIBLE_VERSION_OFFSET)? > FDT_VERSION
    {
        return Err("FDT header");
    }
    let total = word(input, FDT_TOTAL_SIZE_OFFSET)? as usize;
    let b = input.get(..total).ok_or("FDT extent")?;
    if total < FDT_HEADER_BYTES {
        return Err("FDT size");
    }
    let s = word(b, FDT_STRUCTURE_OFFSET)? as usize;
    let t = word(b, FDT_STRINGS_OFFSET)? as usize;
    let r = word(b, FDT_RESERVATIONS_OFFSET)? as usize;
    let sl = word(b, FDT_STRUCTURE_SIZE_OFFSET)? as usize;
    let tl = word(b, FDT_STRINGS_SIZE_OFFSET)? as usize;
    if s < FDT_HEADER_BYTES
        || t < FDT_HEADER_BYTES
        || r < FDT_HEADER_BYTES
        || !s.is_multiple_of(CELL_BYTES)
        || !r.is_multiple_of(WIDE_BYTES)
    {
        return Err("FDT alignment");
    }
    let se = s.checked_add(sl).ok_or("structure overflow")?;
    let te = t.checked_add(tl).ok_or("strings overflow")?;
    let structure = b.get(s..se).ok_or("structure extent")?;
    let strings = b.get(t..te).ok_or("strings extent")?;
    if s < te && t < se {
        return Err("overlapping blocks");
    }
    let mut out = Description {
        ram: Region::default(),
        uart: Region::default(),
        distributor: Region::default(),
        redistributor: Region::default(),
        timer_irq: 0,
        reserved: [Region::default(); MAX_RESERVED_REGIONS],
        reserved_count: 0,
        cpu_affinities: [0; MAX_BOOT_CPUS],
        cpu_count: 0,
        psci_smc: false,
    };
    let mut rp = r;
    loop {
        let base = wide(b, rp)?;
        let size = wide(b, rp + WIDE_BYTES)?;
        rp = rp.checked_add(REGION_BYTES).ok_or("reserve overflow")?;
        if base == 0 && size == 0 {
            break;
        }
        if out.reserved_count == MAX_RESERVED_REGIONS {
            return Err("reserve capacity");
        }
        let rr = Region { base, size };
        rr.end()?;
        out.reserved[out.reserved_count] = rr;
        out.reserved_count += 1;
    }
    if (r < se && s < rp) || (r < te && t < rp) {
        return Err("reserve overlap");
    }
    let mut nodes = [Node::default(); MAX_NODE_DEPTH];
    let mut depth = 0;
    let mut p = 0;
    let mut root_seen = false;
    let mut address_cells = false;
    let mut size_cells = false;
    while p < structure.len() {
        let token = word(structure, p)?;
        p += CELL_BYTES;
        match token {
            FDT_BEGIN_NODE => {
                if depth == MAX_NODE_DEPTH || (depth == 0 && root_seen) {
                    return Err("node depth/root");
                }
                let name = cstr(structure.get(p..).ok_or("node name")?)?;
                if depth == 0 && !name.is_empty() {
                    return Err("root name");
                }
                let reserved =
                    (depth > 0 && nodes[depth - 1].reserved) || name == b"reserved-memory";
                nodes[depth] = Node {
                    name,
                    reserved,
                    ..Node::default()
                };
                depth += 1;
                root_seen = true;
                p = p
                    .checked_add(name.len() + 1)
                    .and_then(|v| v.checked_add(CELL_BYTES - 1))
                    .ok_or("name overflow")?
                    & !(CELL_BYTES - 1);
                if p > structure.len() {
                    return Err("name extent");
                }
            }
            FDT_END_NODE => {
                if depth == 0 {
                    return Err("unbalanced node");
                }
                depth -= 1;
                let n = nodes[depth];
                let matches = |v: &[u8]| n.compatible.split(|&c| c == 0).any(|x| x == v);
                if n.name.starts_with(b"cpu@") && !n.disabled {
                    if depth != CPU_NODE_DEPTH {
                        return Err("unsupported CPU topology");
                    }
                    let parent = nodes[depth - 1];
                    if parent.name != b"cpus"
                        || n.device_type != b"cpu\0"
                        || parent.size_cells != Some(CPU_SIZE_CELLS)
                        || !matches!(
                            parent.address_cells,
                            Some(CPU_ADDRESS_CELLS_NARROW | ADDRESS_SIZE_CELLS)
                        )
                        || n.reg.len() != parent.address_cells.unwrap_or(0) as usize * CELL_BYTES
                        || n.enable_method != b"psci\0"
                        || out.cpu_count == out.cpu_affinities.len()
                    {
                        return Err("unsupported CPU topology");
                    }
                    let affinity = match n.reg.len() {
                        CELL_BYTES => u64::from(word(n.reg, 0)?),
                        WIDE_BYTES => wide(n.reg, 0)?,
                        _ => return Err("CPU affinity width"),
                    };
                    if affinity & !MPIDR_AFFINITY_MASK != 0
                        || out.cpu_affinities[..out.cpu_count].contains(&affinity)
                    {
                        return Err("CPU affinity");
                    }
                    out.cpu_affinities[out.cpu_count] = affinity;
                    out.cpu_count += 1;
                }
                if matches(b"arm,psci-0.2") || matches(b"arm,psci-1.0") {
                    if out.psci_smc || n.method != b"smc\0" {
                        return Err("PSCI conduit");
                    }
                    out.psci_smc = true;
                }
                let relevant = n.name.starts_with(b"memory@")
                    || matches(b"arm,pl011")
                    || matches(b"arm,gic-v3")
                    || matches(b"arm,armv8-timer");
                if relevant && (depth != 1 || n.disabled) {
                    return Err("unsupported device topology/status");
                }
                if n.name.starts_with(b"memory@") {
                    if out.ram.size != 0 || n.reg.len() != REGION_BYTES {
                        return Err("RAM topology");
                    }
                    out.ram = region(n.reg, 0)?;
                }
                if matches(b"arm,pl011") {
                    if out.uart.size != 0 {
                        return Err("duplicate UART");
                    }
                    out.uart = region(n.reg, 0)?;
                }
                if matches(b"arm,gic-v3") {
                    if out.distributor.size != 0 {
                        return Err("duplicate GIC");
                    }
                    out.distributor = region(n.reg, 0)?;
                    out.redistributor = region(n.reg, REGION_BYTES)?;
                }
                if matches(b"arm,armv8-timer") {
                    if n.irq.len() < PHYSICAL_TIMER_SPECIFIER + IRQ_SPECIFIER_BYTES
                        || word(n.irq, PHYSICAL_TIMER_SPECIFIER)? != GIC_IRQ_TYPE_PPI
                        || word(n.irq, PHYSICAL_TIMER_SPECIFIER + 2 * CELL_BYTES)?
                            & GIC_IRQ_TRIGGER_MASK
                            != GIC_IRQ_LEVEL_HIGH
                    {
                        return Err("timer binding");
                    }
                    out.timer_irq = word(n.irq, PHYSICAL_TIMER_SPECIFIER + CELL_BYTES)?
                        .checked_add(GIC_PPI_BASE)
                        .ok_or("IRQ overflow")?;
                }
                if n.reserved && n.name != b"reserved-memory" && !n.reg.is_empty() {
                    if n.reg.len() % REGION_BYTES != 0 {
                        return Err("reserved reg");
                    }
                    for off in (0..n.reg.len()).step_by(REGION_BYTES) {
                        if out.reserved_count == MAX_RESERVED_REGIONS {
                            return Err("reserve capacity");
                        }
                        out.reserved[out.reserved_count] = region(n.reg, off)?;
                        out.reserved_count += 1;
                    }
                }
            }
            FDT_PROPERTY => {
                if depth == 0 {
                    return Err("property outside node");
                }
                let len = word(structure, p)? as usize;
                let offset = word(structure, p + CELL_BYTES)? as usize;
                p += PROPERTY_HEADER_BYTES;
                let end = p.checked_add(len).ok_or("property overflow")?;
                let data = structure.get(p..end).ok_or("property extent")?;
                p = end.checked_add(CELL_BYTES - 1).ok_or("padding overflow")? & !(CELL_BYTES - 1);
                if p > structure.len() {
                    return Err("padding extent");
                }
                let name = cstr(strings.get(offset..).ok_or("property name")?)?;
                let n = &mut nodes[depth - 1];
                if n.property_count == MAX_NODE_PROPERTIES
                    || n.properties[..n.property_count].contains(&name)
                {
                    return Err("duplicate/excess properties");
                }
                n.properties[n.property_count] = name;
                n.property_count += 1;
                if depth == 1 && name == b"#address-cells" {
                    address_cells = true;
                }
                if depth == 1 && name == b"#size-cells" {
                    size_cells = true;
                }
                match name {
                    b"status" => n.disabled = data != b"okay\0" && data != b"ok\0",
                    b"compatible" => {
                        if data.last() != Some(&0) {
                            return Err("compatible terminator");
                        }
                        n.compatible = data;
                    }
                    b"reg" => n.reg = data,
                    b"enable-method" => n.enable_method = data,
                    b"method" => n.method = data,
                    b"device_type" => n.device_type = data,
                    b"#address-cells" => {
                        if len != CELL_BYTES {
                            return Err("address cells width");
                        }
                        let value = word(data, 0)?;
                        n.address_cells = Some(value);
                        if (depth == 1 || n.reserved) && value != ADDRESS_SIZE_CELLS {
                            return Err("unsupported cells");
                        }
                    }
                    b"#size-cells" => {
                        if len != CELL_BYTES {
                            return Err("size cells width");
                        }
                        let value = word(data, 0)?;
                        n.size_cells = Some(value);
                        if (depth == 1 || n.reserved) && value != ADDRESS_SIZE_CELLS {
                            return Err("unsupported cells");
                        }
                    }
                    b"interrupts" => n.irq = data,
                    _ => (),
                }
            }
            FDT_NOP => (),
            FDT_END => {
                if depth != 0
                    || !root_seen
                    || !address_cells
                    || !size_cells
                    || out.ram.size == 0
                    || out.uart.size < PL011_REGISTER_SIZE
                    || out.distributor.size < GICD_REGISTER_SIZE
                    || out.redistributor.size < GICR_FRAME_SIZE
                    || !(GIC_PPI_BASE..GIC_PPI_END).contains(&out.timer_irq)
                {
                    return Err("incomplete platform");
                }
                if structure[p..].iter().any(|&v| v != 0) {
                    return Err("trailing structure");
                }
                return Ok(out);
            }
            _ => return Err("unknown token"),
        }
    }
    Err("missing FDT end")
}
