//! Embeds the application icon and version info in the Windows executable, so
//! the taskbar, Explorer and the installer show the real icon.

fn main() {
    println!("cargo:rerun-if-changed=assets/appicon/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/appicon/icon.ico");
        res.set("ProductName", "PR Dashboard");
        res.set("FileDescription", "PR Dashboard");
        if let Err(e) = res.compile() {
            // Not fatal: the app works without an embedded icon.
            println!("cargo:warning=could not embed the Windows icon: {e}");
        }
    }
}
