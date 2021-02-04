use delay_timer::prelude::*;
use std::thread::sleep;
use std::time::Duration;

#[derive(Default)]
struct RequstBody {
    _id: u64,
    cron_expression: String,
    _token: String,
}

impl RequstBody {
    fn fake_request_body() -> Self {
        let string_buf = [16u16, 16u16, 16u16].repeat(10000);
        let cron_expression = String::from_utf16(&string_buf).unwrap();
        Self {
            cron_expression,
            ..Default::default()
        }
    }
}

impl Into<CandyCronStr> for RequstBody {
    fn into(self) -> CandyCronStr {
        CandyCronStr(self.cron_expression)
    }
}

// LD_PRELOAD=../../tools-bin/libmemory_profiler.so ./target/debug/examples/profile_memory
// ../../tools-bin/memory-profiler-cli server memory-profiling_*.dat
fn main() {
    let capacity: usize = 256_00;
    let mut task_builder_vec: Vec<TaskBuilder> = Vec::with_capacity(capacity);

    for _ in 0..capacity {
        task_builder_vec.push({
            let mut task_builder = TaskBuilder::default();
            task_builder
                .set_frequency_by_candy(CandyFrequency::Repeated(RequstBody::fake_request_body()));

            task_builder
        });
    }

    sleep(Duration::from_secs(25));

    for _ in 0..capacity {
        task_builder_vec.pop().unwrap().free();
    }

    drop(task_builder_vec);

    dbg!("after drop");
}
