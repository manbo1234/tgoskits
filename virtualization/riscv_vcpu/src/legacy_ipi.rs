use crate::RiscvVmExit;

pub(crate) fn decode_legacy_send_ipi_exit(
    hart_mask_ptr: usize,
    copy_from_guest_va: impl FnMut(usize, &mut [u8]) -> usize,
) -> Result<RiscvVmExit, riscv_sbi_ipi::HartMaskReadError> {
    let hart_mask = riscv_sbi_ipi::read_hart_mask(hart_mask_ptr, copy_from_guest_va)?;

    Ok(RiscvVmExit::SendIPI {
        target_cpu: hart_mask as u64,
        target_cpu_aux: 0,
        send_to_all: false,
        send_to_self: false,
        vector: riscv_sbi_ipi::SUPERVISOR_SOFT_CAUSE as u64,
    })
}
