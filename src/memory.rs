use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

// A FrameAllocator that returns usable frames from the bootloader's memory map
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // Get the usable regions from the memory map
        let regions = self.memory_map.iter();

        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        // Map each region to the respective adress range
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        // Transform to an iterator of frame start addresses
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));

        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}
unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}
/// Returns a mutable reference to the given active level 4 table.
///
/// This function must be unsafe because the caller must guartee that complete physical memory is
/// mapped to virtual memory at the passed offset. Also, this function *MUST* only be called once
/// to avoid aliasing '&mut' refrences
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr // very unsafe
}

/// Creates an example mapping for the given page to frame `0xb8000`
pub fn create_example_maping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    use x86_64::structures::paging::PageTableFlags as Flags;

    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = Flags::PRESENT | Flags::WRITABLE;

    let map_to_result = unsafe { mapper.map_to(page, frame, flags, frame_allocator) };
    map_to_result.expect("Map_to failed").flush();
}
/// A frame allocator that always returns `None`
pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        None
    }
}
/// Intialize a new OffsetPageTable.
///
/// This function must be unsfae because the caller must guarantee that the complet phyiscal memory
/// is mapped to virtual memory at the passed 'physical_memory_offset'. This function can also only
/// be called once to avoid aliasing `&mut` references.
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}
/// Translates the given virtual adress to the mapped phyiscial adress, or 'None' if it is not
/// mapped
///
///  This function must be unsafe because the caller must guartee that complete physical memory is
/// mapped to virtual memory at the passed offset.
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    translate_addr_inner(addr, physical_memory_offset)
}

/// Private Function called
/// by 'translate_addr'
///
/// This function is safe to limit the scope of our 'unsafe' blocks, because Rust treats the whole
/// body of unsafe functions as an unsafe block
///
fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    use x86_64::registers::control::Cr3;
    use x86_64::structures::paging::page_table::FrameError;

    // Reads the current level 4 frame from the `CR3` register
    let (level_4_table_frame, _) = Cr3::read();

    let table_index = [
        addr.p4_index(),
        addr.p3_index(),
        addr.p2_index(),
        addr.p1_index(),
    ];
    let mut frame = level_4_table_frame;

    // traverse the multi level page table
    for &index in &table_index {
        // Convert the frame into a page table refrence
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe { &*table_ptr };
        // read the page table entry and update frame
        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge frame not supported"),
        };
    }
    // Calculate the phys adress by adding the offset
    Some(frame.start_address() + u64::from(addr.page_offset()))
}
