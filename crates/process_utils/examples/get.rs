use process_utils::get_sysinfo;

fn main() {
    match get_sysinfo() {
        Some(sysinfo) => println!("Sysinfo: {:#?}", sysinfo),
        None => println!("Could not get sysinfo"),
    }
}
