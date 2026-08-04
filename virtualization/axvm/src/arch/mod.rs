//! Target architecture selection and stable internal dispatch.

pub(crate) use crate::architecture::*;
use crate::{
    AxVmResult,
    architecture::{BootImagePlatform, GuestBootPlatform, HostTimePlatform},
};

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "loongarch64")]
mod loongarch64;
#[cfg(target_arch = "riscv64")]
mod riscv64;
#[cfg(target_arch = "x86_64")]
mod x86_64;

#[cfg(target_arch = "aarch64")]
pub(crate) use aarch64::Aarch64Arch as CurrentArch;
#[cfg(target_arch = "aarch64")]
pub use aarch64::ImageLoader;
#[cfg(target_arch = "aarch64")]
pub(crate) use aarch64::fdt;
#[cfg(target_arch = "loongarch64")]
pub(crate) use loongarch64::LoongArch64Arch as CurrentArch;
#[cfg(target_arch = "loongarch64")]
pub(crate) use loongarch64::boot as guest_platform;
#[cfg(target_arch = "loongarch64")]
pub use loongarch64::boot::ImageLoader;
#[cfg(target_arch = "loongarch64")]
pub(crate) use loongarch64::fdt;
#[cfg(not(target_arch = "loongarch64"))]
pub(crate) mod guest_platform {
    #[doc(hidden)]
    pub const SUPPORTED: bool = false;
}
#[cfg(target_arch = "riscv64")]
pub use riscv64::ImageLoader;
#[cfg(target_arch = "riscv64")]
pub(crate) use riscv64::Riscv64Arch as CurrentArch;
#[cfg(target_arch = "riscv64")]
pub(crate) use riscv64::fdt;
#[cfg(target_arch = "x86_64")]
pub(crate) use x86_64::X86_64Arch as CurrentArch;
#[cfg(target_arch = "x86_64")]
pub use x86_64::boot::ImageLoader;
#[cfg(target_arch = "x86_64")]
pub(crate) use x86_64::fdt;

/// Architecture-specific public compatibility exports.
pub mod platform {
    #[cfg(target_arch = "aarch64")]
    pub use super::aarch64::{host_fdt_bootarg, host_phys_to_virt};
    #[cfg(target_arch = "loongarch64")]
    pub use super::loongarch64::irq::{
        register_guest_irq_route as register_loongarch_guest_irq_route,
        unregister_guest_irq_routes as unregister_loongarch_guest_irq_routes,
    };
    #[cfg(target_arch = "loongarch64")]
    pub use super::loongarch64::{host_fdt_bootarg, host_phys_to_virt};
    #[cfg(target_arch = "riscv64")]
    pub use super::riscv64::{host_fdt_bootarg, host_phys_to_virt};
    #[cfg(target_arch = "x86_64")]
    pub use super::x86_64::irq::{
        register_ioapic_irq_forwarding_activator as register_x86_ioapic_irq_forwarding_activator,
        register_ioapic_irq_forwarding_route as register_x86_ioapic_irq_forwarding_route,
        register_ioapic_irq_forwarding_route_with_trigger as register_x86_ioapic_irq_forwarding_route_with_trigger,
    };
    #[cfg(all(
        any(
            target_arch = "aarch64",
            target_arch = "x86_64",
            target_arch = "loongarch64"
        ),
        any(feature = "fs", feature = "host-fs")
    ))]
    pub use crate::host::arceos::shutdown_host_filesystems;
}

pub(crate) type ArchVCpu = <CurrentArch as ArchOps>::VCpu;
pub(crate) type ArchPerCpu = <CurrentArch as ArchOps>::PerCpu;
pub(crate) type ArchNestedPageTable = <CurrentArch as ArchOps>::NestedPageTable;

pub(crate) fn register_timer_source(
    deadline_source: alloc::sync::Arc<crate::timer::PublishedTimerDeadline>,
    notify: alloc::sync::Arc<ax_std::os::arceos::modules::ax_task::IrqNotify>,
) {
    CurrentArch::register_timer_source(deadline_source, notify);
}

pub(crate) fn request_timer_deadline(deadline_ns: u64) {
    CurrentArch::request_timer_deadline(deadline_ns);
}

pub(crate) fn init_guest_boot_resources() {
    CurrentArch::init_guest_boot_resources();
}

pub(crate) fn prepare_guest_boot(
    vm_config: &mut crate::config::AxVMConfig,
    vm_create_config: &mut axvmconfig::GuestConfig,
    provider: &dyn crate::boot::BootImageProvider,
) -> AxVmResult<Option<crate::boot::fdt::GuestDtbImage>> {
    CurrentArch::prepare_guest_boot(vm_config, vm_create_config, provider)
}

pub(crate) fn load_images_from_memory(
    loader: &mut crate::boot::images::ImageLoaderCore<'_>,
    images: crate::boot::StaticVmImage,
) -> AxVmResult {
    CurrentArch::load_images_from_memory(loader, images)
}

#[cfg(any(feature = "fs", feature = "host-fs"))]
pub(crate) fn load_images_from_filesystem(
    loader: &mut crate::boot::images::ImageLoaderCore<'_>,
) -> AxVmResult {
    CurrentArch::load_images_from_filesystem(loader)
}

pub(crate) fn is_x86_linux_image_config(
    config: &axvmconfig::GuestConfig,
    provider: &dyn crate::boot::BootImageProvider,
) -> bool {
    CurrentArch::is_x86_linux_image_config(config, provider)
}

pub(crate) fn default_boot_firmware_load_gpa(
    config: &axvmconfig::GuestConfig,
) -> Option<axvm_types::GuestPhysAddr> {
    CurrentArch::default_boot_firmware_load_gpa(config)
}

#[cfg(any(target_arch = "riscv64", test))]
pub(crate) fn legacy_hart_mask_targets(
    hart_mask: usize,
    vcpu_mappings: impl IntoIterator<Item = (usize, Option<usize>, usize)>,
) -> crate::CpuMask<64> {
    let mut targets = crate::CpuMask::new();

    for (vcpu_id, _, phys_id) in vcpu_mappings {
        if phys_id < usize::BITS as usize && ((hart_mask >> phys_id) & 1) != 0 {
            targets.set(vcpu_id, true);
        }
    }

    targets
}

#[cfg(test)]
mod legacy_hart_mask_tests {
    use super::*;

    #[test]
    fn legacy_hart_mask_routes_guest_hart_to_local_vcpu() {
        let mappings = [
            (0usize, Some(1usize << 4), 4usize),
            (1usize, Some(1usize << 5), 5usize),
        ];

        let targets = legacy_hart_mask_targets(1usize << 5, mappings);

        assert!(!targets.get(0));
        assert!(targets.get(1));
        assert!(!targets.get(4));
        assert!(!targets.get(5));
    }

    #[test]
    fn legacy_hart_mask_ignores_unmapped_guest_hart_bits() {
        let mappings = [
            (0usize, Some(1usize << 4), 4usize),
            (1usize, Some(1usize << 5), 5usize),
        ];

        let targets = legacy_hart_mask_targets(1usize << 6, mappings);

        assert!(targets.is_empty());
    }
}
