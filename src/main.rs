#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(simpleosrust::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use simpleosrust::println;
// static HELLO: &[u8] = b"Hello World!";
entry_point!(kernel_main);
/// This fucntion acts as the entry point, since the linker will look for one, and since we use #[no_main], we need to define one ourselves
/// Normally, we would also need to use `#[no_mangle]`, to prevent the compiler from mangling the
/// name of the function, however, the 'entry_point' macro does this for us.
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Hello World!"); // This uses the println! macro from src/vga_buffer.rs that we defined
    use simpleosrust::memory;
    use simpleosrust::memory::BootInfoFrameAllocator;
    use x86_64::{
        structures::paging::{Page, Translate},
        VirtAddr,
    };

    simpleosrust::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

    // Intialize a new mapper
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    // Map an unused page
    let page = Page::containing_address(VirtAddr::new(0));
    memory::create_example_maping(page, &mut mapper, &mut frame_allocator);

    // Write the string `New!` to the screen through the new mapping
    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e) };
    let addresses = [
        // the identity-mapped vga buffer page
        0xb8000,
        // some code page
        0x201008,
        // some stack page
        0x0100_0020_1a10,
        // virtual address mapped to physical address 0
        boot_info.physical_memory_offset,
    ];
    for &adress in &addresses {
        let virt = VirtAddr::new(adress);
        let phys = mapper.translate_addr(virt);
        println!("{:?} -> {:?}", virt, phys);
    }
    #[cfg(test)]
    test_main(); // Entry point for custom tests

    println!("It did not crash!");
    simpleosrust::hlt_loop();
}

/// This function is called on panic, but only if we are not testing
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info); // Prints the panic info
    simpleosrust::hlt_loop();
}
/// This version of the panic function *is* called on panic, but only if we are testing.
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    simpleosrust::test_panic_handler(info) // From srs/lib.rs
}
