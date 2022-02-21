# Version 0.11.1

## Changed

- Optimize internal code for better clarity and expression.

- Fixed a bug where task execution time is unstable.([#36](https://github.com/BinChengZhao/delay-timer/issues/36))

### Details

# Version 0.11.0 

## Changed
- Optimized the entire delay-timer usage by removing macros for creating asynchronous tasks and now allowing users to use a closure that returns a Future.

- The default internal runtime currently used by 'Tokio', 'smol' can also be manually enabled by the user.

- Updated lots of utility cases, lots of documentation, and dependency updates.

- Fixed a bug where tasks were executed ahead of time.([#33](https://github.com/BinChengZhao/delay-timer/issues/33))

### Details

Major refactoring of 'task execution'.

# Version 0.10.1 

## Changed
Fix a strange compilation error that was leaked by the compiler. ([#30](https://github.com/BinChengZhao/delay-timer/issues/30)), thanks `elderbig` !

# Version 0.10.0 

## Changed
Optimized the use of internal `time-wheel` memory. ([#28](https://github.com/BinChengZhao/delay-timer/issues/28)), thanks `elderbig` !

### Details
There is a `time-wheel` in `delay-timer`, which is the carrier of all tasks.

The time wheel uses slots (time scales) as units, each slot corresponds to a hash table, when a slot is rotated to it will execute the task that is already ready internally, when the task is executed it will move from one slot to another. In order to have enough capacity to store the tasks, there may be a memory allocation here, so that by the time the whole time wheel is traversed, each internal time wheel-slot will have rich memory capacity, and when there are many tasks the memory occupied by the whole time wheel will be very large. So it will be necessary to shrink the memory in time.

This change is to shrink the memory in time after each round of training slots and executing tasks to ensure that the slots have a basic and compact capacity.

# Version 0.9.2

## Changed
Add a new Error enumeration `CommandChildError`!

# Version 0.9.1 

## Changed
Tweak the compile script ([#24](https://github.com/BinChengZhao/delay-timer/issues/24)), thanks `chaaz` !

# Version 0.9.0 

v0.9.0 New features:


    1. Add more user-friendly api for better experience!
    
      1.1. Gradually deprecating TaskBuilder api `set_frequency` & `set_frequency_by_candy`.
      
      1.2. Add more user-friendly api such as `set_frequency_once_by_timestamp_seconds` | `set_frequency_count_down_by_days` | `set_frequency_repeated_by_cron_str` these having better experience.
    

Update dependency :

    Update cron_clock .

Update examples:

    Change, async-std & tokio & demo & generic & increase use cases.

Enriched Unit tests & documentation.
# Version 0.8.2

v0.8.2 New features:

    1. FIXED:
         When the `TimeoutTask` event fails to remove the handle, Ok(()) is returned by default.
         This causes the `TimeoutTask` event to be sent to the outside world by `status_report_sender`,Which is a buggy behavior.
# Version 0.8.1

v0.8.1 New features:

    1. Optimization logic: After task removal, the surviving task instance resources can be cancelled normally.

# Version 0.8.0

v0.8.0 New features:

    1. Optimized the api for custom tokio runtime, better api definition, better experience.

      Api changes.
        1.  Added: `tokio_runtime_by_default` & `tokio_runtime_by_custom` & `tokio_runtime_shared_by_custom` .

        2. Private: `tokio_runtime`.
  
    2. Optimized the method of canceling task instances, no error log is recorded when canceling a task failure after timeout.

    3. Fix the compile error about `quit_one_task_handler` under nightly version.
# Version 0.7.0

v0.7.0 New features:

    1. Fix the bugs before V0.6.1, When receive a timeout event from the `sweeper`, it is possible that the task has been completed.
    So can't just call cancel.

      Two options fix it.
        1. `sweeper` Remove the recycling unit when the task is finished or cancelled.(Because rust's `BinaryHeap` does not support removing specific elements, this scheme is not used.)

        2. The public event is sent only after the internal event has been processed and succeeded. (Adopted.)
  
    2. Adjust api names (runnable -> runnable, maximun -> maximum).

# Version 0.6.1

v0.6.1 New features:


    1. Optimize `sweeper` performance and recycle expired tasks more efficiently.

# Version 0.6.0 

v0.6.0 New features:


    1. Grouping errors associated with Task and TaskInstance and implementing the standard Error-Trait.
    
    2. Add `advance_task`-api to support users to manually trigger the execution of tasks..
    
    3. Add `get_*` api to `PublicFinishTaskBody` to support getting task_id or record_id or finish_time.

# Version 0.5.0 

v0.5.0 New features:


    1. Remove all unwrap() calls.
    
    2. Redefined the errors exposed to the outside (encapsulated by `thiserror`) as libs to never expose `anyhow::Error` to the outside.
    
      2.1. Set separate `TaskError`. for `Task`-related operations.
      2.2. Set separate `TaskInstanceError`. for `Task-Instance`-related operations.
    
    
    3. Optimized shell command parsing function, set exclusive pipeline for derived subprocesses -stderr.


Update dependency :
    Add, `thiserror`.


Enriched documentation.


# Version 0.4.0 

v0.4.0 New features:


    1. Support dynamic modification of running tasks.
    2. Support get handle `TaskInstancesChain` after insert task, and get running task instance `TaskInstance` dynamically.
    
      2.1. The task instance of a running task can be dynamically cancelled.
      2.2. There are three types of cancellation: synchronous blocking cancellation, time-out-limited cancellation, and asynchronous cancellation.
      2.3. Support reading the running status of running tasks.
    
    3. Support to get the output of internal asynchronous subtask process.

Update dependency :

    Replace waitmap -> dashmap .
    Update cron_clock .

Update examples:

    Add, async-std & tokio use cases.
    Add, dynamically cancel running task example case.

Enriched documentation.



# Version 0.3.0 

- Stable to stable-rustc compilation, with repair optimization.

1.Compilable at stable by conditional compilation.

2.Balancing performance and user experience (ajust CandyCronStr inner-type and add free-api for TaskBuilder).

3.Support custom setting of time zone.

4.Fix the clock too fast issue.

5.Use `next_second_hand` to solve a schedule problem.

- 
# Version 0.2.0

- Add `tokio-support` and `status-report`  features, support for tokio ecology, internal logic optimization, generate tasks faster, add syntactic sugar to cron-expressions, etc.

1.Enriched a large number of documents, more easy to use.

2.tokio-Runtime is supported.

3.Custom syntactic sugar for Cron expressions is supported, and the API is more friendly.

4.Optimize the internal logic, more secure execution.

5.task supports new features, you can set the maximum number of parallelism, and the task can automatically recycle the handle after completion.

6.Support status reporting, you can get the internal time by DelayTImer, and you can use the Cancel running task API now.

7.Generate more powerful macros for asynchronous task Body, more details you can find in the documentation and examples.

# Version 0.1.0

- delay-timer is a task manager based on a time wheel algorithm, which makes it easy to manage timed tasks, or to periodically execute arbitrary tasks such as closures.

The underlying runtime is currently based on smol, so upper level applications that want to extend asynchronous functionality need to use libraries that are compatible with smol.

Since the library currently includes features such as #[bench], it needs to be developed in a nightly version.
