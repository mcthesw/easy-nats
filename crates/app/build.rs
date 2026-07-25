fn main() {
    compile_windows_resources();
}

#[cfg(windows)]
fn compile_windows_resources() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../assets/icons/easy-nats.ico");
    if let Err(e) = res.compile() {
        eprintln!("cargo:warning=winresource failed: {e}");
    }
}

#[cfg(not(windows))]
fn compile_windows_resources() {}
