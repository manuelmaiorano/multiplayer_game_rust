

pub fn get_current_time() -> f32{
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("").as_secs_f32()

}