pub(self) mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn get_built_info() -> String {
    format!(
        "Version: {}.{}.{}\nAuthors: {}\nDescription: {}\nGit commit hash: {}\nTarget: {}\nHost: {}, \nRustc: {}",
        built_info::PKG_VERSION_MAJOR,
        built_info::PKG_VERSION_MINOR,
        built_info::PKG_VERSION_PATCH,
        built_info::PKG_AUTHORS,
        built_info::PKG_DESCRIPTION,
        built_info::GIT_COMMIT_HASH.unwrap_or("None"),
        built_info::TARGET,
        built_info::HOST,
        built_info::RUSTC_VERSION
    )
}

pub fn print_built_info() {
    println!("{}", get_built_info());
}