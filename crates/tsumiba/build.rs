//! Embeds the icon and version information into `tsumiba.exe` on Windows, so
//! Explorer, the taskbar and the "Apps" list show the real icon and name.
//! On every other target this does nothing.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../assets/windows/tsumiba.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_windows_resources();
    }
}

#[cfg(windows)]
fn embed_windows_resources() {
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon("../../assets/windows/tsumiba.ico")
        .set("ProductName", "tsumiba")
        .set("FileDescription", "tsumiba, a memo and file shelf")
        .set("CompanyName", "hjosugi")
        .set("LegalCopyright", "MIT License");
    // A machine without the Windows SDK's resource compiler can still build a
    // working program; it just has the default icon.
    if let Err(error) = resource.compile() {
        println!("cargo:warning=could not embed the Windows icon and version info: {error}");
    }
}

// Cross-compiling to Windows from another OS is not something the release
// pipeline does, and the resource compiler is a Windows tool.
#[cfg(not(windows))]
fn embed_windows_resources() {}
