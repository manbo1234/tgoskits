use riscv_sbi_ipi::{
    HartMaskReadError, SUPERVISOR_EXTERNAL_CAUSE, SUPERVISOR_SOFT_CAUSE, SUPERVISOR_TIMER_CAUSE,
    SupervisorInterruptAction, VSEIP_HVIP_BIT, VSSIP_HVIP_BIT, VSTIP_HVIP_BIT,
    classify_supervisor_interrupt, clear_virtual_soft_pending, read_hart_mask, select_targets,
};

#[test]
fn guest_memory_word_routes_only_masked_harts() {
    let mask = 0b1010usize;
    let mask_bytes = mask.to_ne_bytes();

    let hart_mask = read_hart_mask(0x4000, |guest_va, bytes| {
        assert_eq!(guest_va, 0x4000);
        bytes.copy_from_slice(&mask_bytes);
        bytes.len()
    })
    .unwrap();

    assert_eq!(select_targets(hart_mask, [0, 1, 2, 3]), vec![1, 3]);
}

#[test]
fn mask_bit_one_selects_only_vcpu_one() {
    assert_eq!(select_targets(1 << 1, [0, 1, 2, 3]), vec![1]);
}

#[test]
fn mask_bits_zero_and_two_select_only_vcpus_zero_and_two() {
    assert_eq!(
        select_targets((1 << 0) | (1 << 2), [0, 1, 2, 3]),
        vec![0, 2]
    );
}

#[test]
fn invalid_guest_mask_pointer_rejects_before_routing() {
    let err = read_hart_mask(0, |_guest_va, _bytes| 0).unwrap_err();

    assert_eq!(
        err,
        HartMaskReadError::ShortRead {
            expected: core::mem::size_of::<usize>(),
            copied: 0,
        }
    );
}

#[test]
fn clear_ipi_clears_only_vssip() {
    let hvip = 1usize << VSSIP_HVIP_BIT;
    assert_eq!(clear_virtual_soft_pending(hvip), 0);
}

#[test]
fn clear_ipi_preserves_timer_and_external_pending() {
    let hvip = (1usize << VSSIP_HVIP_BIT) | (1usize << VSTIP_HVIP_BIT) | (1usize << VSEIP_HVIP_BIT);

    let cleared = clear_virtual_soft_pending(hvip);

    assert_eq!(cleared & (1usize << VSSIP_HVIP_BIT), 0);
    assert_ne!(cleared & (1usize << VSTIP_HVIP_BIT), 0);
    assert_ne!(cleared & (1usize << VSEIP_HVIP_BIT), 0);
}

#[test]
fn host_supervisor_soft_is_consumed_by_host() {
    assert_eq!(
        classify_supervisor_interrupt(SUPERVISOR_SOFT_CAUSE),
        Some(SupervisorInterruptAction::ConsumeHostSoft)
    );
}

#[test]
fn timer_and_external_are_guest_interrupts() {
    assert_eq!(
        classify_supervisor_interrupt(SUPERVISOR_TIMER_CAUSE),
        Some(SupervisorInterruptAction::InjectGuestTimer)
    );
    assert_eq!(
        classify_supervisor_interrupt(SUPERVISOR_EXTERNAL_CAUSE),
        Some(SupervisorInterruptAction::InjectGuestExternal)
    );
}
