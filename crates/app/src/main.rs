#![windows_subsystem = "windows"]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    easy_nats::run_native();
}
