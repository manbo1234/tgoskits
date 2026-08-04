#![no_std]

extern crate alloc;

use alloc::vec::Vec;

pub const VSSIP_HVIP_BIT: usize = 2;
pub const VSTIP_HVIP_BIT: usize = 6;
pub const VSEIP_HVIP_BIT: usize = 10;

pub const SUPERVISOR_SOFT_CAUSE: usize = 1;
pub const SUPERVISOR_TIMER_CAUSE: usize = 5;
pub const SUPERVISOR_EXTERNAL_CAUSE: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HartMaskReadError {
    ShortRead { expected: usize, copied: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorInterruptAction {
    ConsumeHostSoft,
    InjectGuestTimer,
    InjectGuestExternal,
}

pub fn read_hart_mask(
    guest_va: usize,
    mut copy_from_guest_va: impl FnMut(usize, &mut [u8]) -> usize,
) -> Result<usize, HartMaskReadError> {
    let mut mask_bytes = [0u8; core::mem::size_of::<usize>()];
    let copied = copy_from_guest_va(guest_va, &mut mask_bytes);

    if copied != mask_bytes.len() {
        return Err(HartMaskReadError::ShortRead {
            expected: mask_bytes.len(),
            copied,
        });
    }

    Ok(usize::from_ne_bytes(mask_bytes))
}

pub fn select_targets(hart_mask: usize, vcpu_ids: impl IntoIterator<Item = usize>) -> Vec<usize> {
    vcpu_ids
        .into_iter()
        .filter(|&vcpu_id| vcpu_id < usize::BITS as usize && ((hart_mask >> vcpu_id) & 1) != 0)
        .collect()
}

pub fn clear_virtual_soft_pending(hvip: usize) -> usize {
    hvip & !(1usize << VSSIP_HVIP_BIT)
}

pub fn classify_supervisor_interrupt(cause: usize) -> Option<SupervisorInterruptAction> {
    match cause {
        SUPERVISOR_SOFT_CAUSE => Some(SupervisorInterruptAction::ConsumeHostSoft),
        SUPERVISOR_TIMER_CAUSE => Some(SupervisorInterruptAction::InjectGuestTimer),
        SUPERVISOR_EXTERNAL_CAUSE => Some(SupervisorInterruptAction::InjectGuestExternal),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_mask_is_empty_and_not_broadcast() {
        assert_eq!(select_targets(0, [0, 1, 2, 3]), Vec::<usize>::new());
    }

    #[test]
    fn selected_mask_routes_only_selected_harts() {
        assert_eq!(select_targets(0b1010, [0, 1, 2, 3]), alloc::vec![1, 3]);
    }

    #[test]
    fn unreadable_guest_mask_is_rejected() {
        let err = read_hart_mask(0x1000, |_guest_va, _bytes| 0).unwrap_err();
        assert_eq!(
            err,
            HartMaskReadError::ShortRead {
                expected: core::mem::size_of::<usize>(),
                copied: 0,
            }
        );
    }
}
