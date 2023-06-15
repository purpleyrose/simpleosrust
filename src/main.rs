#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(simpleosrust::test_runner)]
#![reexport_test_harness_main = "test_main"]


use core::panic::PanicInfo;
use simpleosrust::println;

// static HELLO: &[u8] = b"Hello World!";

/// This fucntion acts as the entry point, since the linker will look for one, and since we use #[no_main], we need to define one ourselves
/// #[no_mangle] is also used here, because we don't want the compiler to mangle the name of this function, as the linker needs it to be called `_start`.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    println!("Whussup big guy ;)"); // This uses the println! macro from src/vga_buffer.rs that we defined


    simpleosrust::init();

    fn stack_overflow() {
        stack_overflow();
    }

    stack_overflow();
    #[cfg(test)]
    test_main(); // Entry point for custom tests

    println!("It did not crash!");
    loop{}
}

/// This function is called on panic, but only if we are not testing
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info); // Prints the panic info
    loop {}
}
/// This version of the panic function *is* called on panic, but only if we are testing.
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    simpleosrust::test_panic_handler(info) // From srs/lib.rs
}



