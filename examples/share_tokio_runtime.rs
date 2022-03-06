use color_eyre::{eyre::Report, eyre::WrapErr, Section};
use delay_timer::prelude::*;
use tokio::runtime::Runtime;
use tracing::{info, instrument};

fn main() -> Result<(), Report> {
    install_tracing();
    color_eyre::install()?;

    let runtime = Runtime::new()?;
    let shared_runtime = std::sync::Arc::new(runtime);

    shared_runtime.block_on({
        let rt = shared_runtime.clone();

        async move {
            let delay_timer = DelayTimerBuilder::default()
                .tokio_runtime_shared_by_custom(rt)
                .build();
            let mut chain;
            for (id, cron_str) in [(1, "0 1 6 * * * *"), (2, "0 10 6 * * * *")] {
                chain = delay_timer.insert_task(build_task_async_print(id, cron_str)?)?;
                chain.next_with_async_wait().await?;
            }

            Ok::<(), Report>(())
        }
    })?;

    Ok(read_config()?)
}

fn build_task_async_print(id: u64, cron_str: &'static str) -> Result<Task, TaskError> {
    let mut task_builder = TaskBuilder::default();

    let body = move || async move {
        info!("create_async_fn_body:i'success {}", cron_str);
    };

    task_builder
        .set_task_id(id)
        .set_frequency_repeated_by_cron_str(cron_str)
        .set_schedule_iterator_time_zone(ScheduleIteratorTimeZone::Utc)
        .set_maximum_parallel_runnable_num(2)
        .spawn_async_routine(body)
}

fn install_tracing() {
    use tracing_error::ErrorLayer;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("trace"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(ErrorLayer::default())
        .init();
}

#[instrument]
fn read_file(path: &str) -> Result<(), Report> {
    info!("Reading file");
    Ok(std::fs::read_to_string(path).map(drop)?)
}

#[instrument]
fn read_config() -> Result<(), Report> {
    read_file("fake_file")
        .wrap_err("Unable to read config")
        .suggestion("try using a file that exists next time")
}
