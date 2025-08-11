use std::time::{SystemTime, Duration, UNIX_EPOCH};

use humantime::format_duration;

pub fn get_current_time() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch_result = start
        .duration_since(UNIX_EPOCH);

    let since_the_epoch = match since_the_epoch_result {
            Ok(duration) => duration,
            Err(err) => {
                println!("{err}");
                return 0;
            }
        
    };
    
    return since_the_epoch.as_secs() * 1000 +
            since_the_epoch.subsec_nanos() as u64 / 1_000_000;
    
}

pub fn get_duration_human_time(start_ms: u64, end_ms: u64) -> String {
    format_duration(Duration::from_millis(end_ms - start_ms)).to_string()
}