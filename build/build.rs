extern crate winresource;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        /* Set windows specific steps like Icon */
        let mut res = winresource::WindowsResource::new();
        res.set_icon("./build/gman.ico");
        res.compile().unwrap();
    }
}
