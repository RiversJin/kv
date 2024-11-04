fn generate_built_info(){
    built::write_built_file().expect("Failed to acquire build-time information");
}

fn main() {
    generate_built_info();
}