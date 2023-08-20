

pub fn get_current_time() -> f64{
    let start = std::time::SystemTime::now();
    let since_the_epoch = start
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards");

    since_the_epoch.as_secs_f64()

}