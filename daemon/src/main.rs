use tuxphones::pulse::PulseHandle;

fn main() {
    println!("Hello, world!");
    let mut handle = PulseHandle::new().expect("Failed!");
    let sinks = handle.get_sinks();
    sinks.into_iter().for_each(|f| {
        println!("{}", f.name);
    });
}
