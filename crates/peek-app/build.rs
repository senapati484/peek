fn main() {
    println!("cargo:rerun-if-changed=../../ui/dist");
    println!("cargo:rerun-if-changed=../../assets");

    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../assets/app_icon.ico");
        res.set("ProductName", "Peek");
        res.set("FileDescription", "Peek Web Browser");
        let _ = res.compile();
    }
}
