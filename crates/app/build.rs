fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icons/easy-nats.ico");
        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=winresource failed: {e}");
        }
    }
}
