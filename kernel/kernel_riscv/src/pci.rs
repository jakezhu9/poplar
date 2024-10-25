use crate::interrupts;
use alloc::{collections::BTreeMap, sync::Arc, vec, vec::Vec};
use bit_field::BitField;
use core::ptr;
use fdt::Fdt;
use hal::memory::PAddr;
use kernel::{object::event::Event, pci::PciInterruptConfigurator};
use pci_types::{
    capability::{MsiCapability, MsixCapability},
    Bar,
    ConfigRegionAccess,
    PciAddress,
};
use spinning_top::Spinlock;
use tracing::{debug, info};

// TODO: this should have an interrupt guard as well
static INTERRUPT_ROUTING: Spinlock<BTreeMap<u32, Vec<Arc<Event>>>> = Spinlock::new(BTreeMap::new());

pub struct PciAccess {
    start: *const u8,
    size: usize,
    legacy_interrupt_remapping: BTreeMap<(PciAddress, u8), u32>,
}

impl PciAccess {
    pub fn new(fdt: &Fdt) -> Option<PciAccess> {
        let pci_node = fdt
            .all_nodes()
            .filter(|node| {
                node.compatible().map_or(false, |c| {
                    c.all().any(|c| ["pci-host-ecam-generic", "pci-host-cam-generic"].contains(&c))
                })
            })
            .next()?;
        let ecam_window = pci_node.reg().expect("PCI entry doesn't have a reg property").next().unwrap();
        let ecam_address = hal_riscv::platform::kernel_map::physical_to_virtual(
            PAddr::new(ecam_window.starting_address as usize).unwrap(),
        );

        /*
         * Find routing information for legacy interrupt pins from the device tree.
         */
        let remapping = {
            let mut remapping = BTreeMap::new();
            let interrupt_map = pci_node.interrupt_map().unwrap();
            let interrupt_map_mask = pci_node.interrupt_map_mask().unwrap();

            for mapping in interrupt_map {
                let child_address_hi = mapping.child_unit_address_hi & interrupt_map_mask.address_mask_hi;
                let address = PciAddress::new(
                    0,
                    child_address_hi.get_bits(16..24) as u8,
                    child_address_hi.get_bits(11..16) as u8,
                    child_address_hi.get_bits(8..11) as u8,
                );
                let pin = mapping.child_interrupt_specifier & interrupt_map_mask.interrupt_mask;
                // TODO: we need a way of mapping FDT interrupt data to vectors based on the interrupt
                // mode (this should only be done on AIA archs)
                let mapped_interrupt = mapping.parent_interrupt_specifier.get_bits(32..64) as u32;

                debug!(
                    "Legacy PCI interrupt remapping: {:#?}, pin = {} -> {} ({:#x})",
                    address, pin, mapped_interrupt, mapped_interrupt
                );
                crate::interrupts::handle_wired_device_interrupt(
                    mapping.parent_interrupt_specifier,
                    pci_interrupt_handler,
                );

                INTERRUPT_ROUTING.lock().insert(mapped_interrupt, Vec::new());
                remapping.insert((address, pin as u8), mapped_interrupt);
            }
            remapping
        };

        Some(PciAccess {
            start: ecam_address.ptr(),
            size: ecam_window.size.unwrap(),
            legacy_interrupt_remapping: remapping,
        })
    }

    fn address_for(&self, pci_address: PciAddress) -> *const u8 {
        unsafe {
            self.start.add(
                usize::from(pci_address.bus()) << 20
                    | usize::from(pci_address.device()) << 15
                    | usize::from(pci_address.function()) << 12,
            )
        }
    }
}

unsafe impl Send for PciAccess {}

impl ConfigRegionAccess for PciAccess {
    unsafe fn read(&self, address: PciAddress, offset: u16) -> u32 {
        ptr::read_volatile(self.address_for(address).add(offset as usize) as *const u32)
    }

    unsafe fn write(&self, address: PciAddress, offset: u16, value: u32) {
        ptr::write_volatile(self.address_for(address).add(offset as usize) as *mut u32, value);
    }
}

impl PciInterruptConfigurator for PciAccess {
    fn configure_legacy(&self, function: PciAddress, pin: u8) -> Arc<Event> {
        info!("Configuring PCI device to use legacy interrupts: {:?}", function);
        let event = Event::new();

        let remapped_interrupt =
            self.legacy_interrupt_remapping.get(&(function, pin)).expect("PCI interrupt not in remapping!");
        INTERRUPT_ROUTING.lock().get_mut(&remapped_interrupt).unwrap().push(event.clone());

        event
    }

    fn configure_msi(&self, function: PciAddress, msi: &mut MsiCapability) -> Arc<Event> {
        let event = Event::new();
        info!("Configuring PCI device to use MSI interrupts: {:?}", function);

        // TODO: allocate numbers from somewhere???
        // TODO: we need a way to track unused interrupt vectors - can we find the valid range from
        // the device tree and then reserve ones used by other devices or something? (this feels
        // like it could live in the common kernel and be useful for everyone)
        let message_number = 2;
        INTERRUPT_ROUTING.lock().insert(message_number, vec![event.clone()]);

        interrupts::handle_interrupt(message_number as u16, pci_interrupt_handler);

        // TODO: get out of the device tree
        msi.set_message_info(0x28000000, message_number as u32, self);
        msi.set_enabled(true, self);

        event
    }

    fn configure_msix(&self, function: PciAddress, table_bar: Bar, msix: &mut MsixCapability) -> Arc<Event> {
        let event = Event::new();
        info!("Configuring PCI device to use MSI-X interrupts: {:?}", function);

        // TODO: this is bad and we should allocate these for real as per above
        let message_number = 3;
        INTERRUPT_ROUTING.lock().insert(message_number, vec![event.clone()]);

        interrupts::handle_interrupt(message_number as u16, pci_interrupt_handler);

        // TODO: get out of the device tree
        let message_address = 0x28000000;
        msix.set_enabled(true, self);

        let table_base_phys = match table_bar {
            Bar::Memory32 { address, .. } => (address + msix.table_offset()) as usize,
            Bar::Memory64 { address, .. } => address as usize + msix.table_offset() as usize,
            _ => panic!(),
        };
        let table_base_virt =
            hal_riscv::platform::kernel_map::physical_to_virtual(PAddr::new(table_base_phys).unwrap());
        // TODO: offset into the table if we ever need an entry that isn't the first
        let entry_ptr = table_base_virt.mut_ptr() as *mut u32;

        /*
         * Each entry of the MSI-X table is laid out as:
         *    0x00 => Message Address
         *    0x04 => Message Upper Address
         *    0x08 => Message Data
         *    0x0c => Vector Control
         */
        unsafe {
            ptr::write_volatile(entry_ptr.byte_add(0x00), message_address);
            ptr::write_volatile(entry_ptr.byte_add(0x04), 0);
            ptr::write_volatile(entry_ptr.byte_add(0x08), message_number as u32);
            ptr::write_volatile(entry_ptr.byte_add(0x0c), 0);
        }

        event
    }
}
fn pci_interrupt_handler(number: u16) {
    let routing = INTERRUPT_ROUTING.lock();
    if let Some(events) = routing.get(&(number as u32)) {
        for event in events {
            event.signal();
        }
    }
}
