#![forbid(unsafe_code)]
use kernel_core::memory::Error;
pub const PAGE_OFFSET_MASK: u64 = 4095;
pub const PHYSICAL_ADDRESS_LIMIT: u64 = 1 << 40;
pub const OUTPUT_ADDRESS_MASK: u64 = 0x0000fffffffff000;
pub const TABLE_OR_PAGE: u64 = 3;
pub const ACCESS_FLAG: u64 = 1 << 10;
pub const INNER_SHAREABLE: u64 = 3 << 8;
pub const NORMAL_MEMORY_INDEX: u64 = 1 << 2;
pub const READ_ONLY: u64 = 1 << 7;
pub const USER_EXECUTE_NEVER: u64 = 1 << 54;
pub const PRIVILEGED_EXECUTE_NEVER: u64 = 1 << 53;
pub const USER_ACCESS: u64 = 1 << 6;
pub const NOT_GLOBAL: u64 = 1 << 11;
pub fn user_descriptor(pa: u64, writable: bool, executable: bool) -> Result<u64, Error> {
    let kernel = descriptor(pa, writable, executable)?;
    Ok((kernel & !USER_EXECUTE_NEVER)
        | USER_ACCESS
        | NOT_GLOBAL
        | PRIVILEGED_EXECUTE_NEVER
        | if executable { 0 } else { USER_EXECUTE_NEVER })
}
pub fn descriptor(pa: u64, writable: bool, executable: bool) -> Result<u64, Error> {
    if pa & PAGE_OFFSET_MASK != 0 || pa >= PHYSICAL_ADDRESS_LIMIT || (writable && executable) {
        return Err(Error::Invalid);
    }
    Ok(pa
        | TABLE_OR_PAGE
        | ACCESS_FLAG
        | INNER_SHAREABLE
        | NORMAL_MEMORY_INDEX
        | USER_EXECUTE_NEVER
        | if writable { 0 } else { READ_ONLY }
        | if executable {
            0
        } else {
            PRIVILEGED_EXECUTE_NEVER
        })
}
#[cfg(test)]
mod tests {
    const TEST_PAGE_ADDRESS: u64 = 0x4000_0000;
    #[test]
    fn user_permissions_preserve_privilege_and_wx_boundaries() {
        let code = super::user_descriptor(TEST_PAGE_ADDRESS, false, true).unwrap();
        let data = super::user_descriptor(TEST_PAGE_ADDRESS, true, false).unwrap();
        for entry in [code, data] {
            assert_ne!(entry & super::USER_ACCESS, 0);
            assert_ne!(entry & super::NOT_GLOBAL, 0);
            assert_ne!(entry & super::PRIVILEGED_EXECUTE_NEVER, 0);
        }
        assert_ne!(code & super::READ_ONLY, 0);
        assert_eq!(code & super::USER_EXECUTE_NEVER, 0);
        assert_eq!(data & super::READ_ONLY, 0);
        assert_ne!(data & super::USER_EXECUTE_NEVER, 0);
        assert!(super::user_descriptor(TEST_PAGE_ADDRESS, true, true).is_err());
    }
    #[test]
    fn negative_permissions_and_alignment() {
        let descriptor = super::descriptor(TEST_PAGE_ADDRESS, false, false).unwrap();
        assert_eq!(descriptor & super::OUTPUT_ADDRESS_MASK, TEST_PAGE_ADDRESS);
        assert_eq!(descriptor & super::TABLE_OR_PAGE, super::TABLE_OR_PAGE);
        assert!(super::descriptor(TEST_PAGE_ADDRESS, true, true).is_err());
        assert!(super::descriptor(1, false, false).is_err());
        assert!(super::descriptor(super::PHYSICAL_ADDRESS_LIMIT, false, false).is_err());
        assert_ne!(
            super::descriptor(TEST_PAGE_ADDRESS, false, false).unwrap() & super::READ_ONLY,
            0
        );
    }
}
