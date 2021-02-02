use delay_timer::prelude::*;

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
    // let mut task_builder_vec: Vec<RequstBody> = Vec::with_capacity(100_000);
    let mut task_builder_vec: Vec<TaskBuilder> = Vec::with_capacity(100_000);

    for _ in 0..256_00 {
        task_builder_vec.push({
            let mut task_builder = TaskBuilder::default();
            task_builder
                .set_frequency_by_candy(CandyFrequency::Repeated(RequstBody::fake_request_body()));

            task_builder
        });

        // task_builder_vec.push(RequstBody::fake_request_body());
    }

    for _ in 0..256_00 {
        // FIXME: It can't free memory.
        task_builder_vec.pop().unwrap().free();
        // task_builder_vec.pop().unwrap();
    }

    drop(task_builder_vec);
}
