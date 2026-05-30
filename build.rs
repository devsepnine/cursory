fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Cursory");
        res.set("FileDescription", "Cursory");
        res.compile().expect("failed to compile Windows resources");
    }
}
