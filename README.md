# delayTimer
Time-manager of delayed tasks.


## To Do List
- [x] add update-event ,handle_event can use it msg for update timer_core, update-event(task_id, old_slotid, new_slotid, call_at) handle_event can use call_at check if postpone run.
- [x] error handle need supplement.
- [x] neaten todo in code, replenish tests and benchmark.
- [x] when append that generate async-wrapper macro, ready for release.
- [x] append that generate bloking-wrapper macro, ready for release.
- [x] batch-opration.
- [x] report-for-server.
- [x] TASK-TAG.


#### The author comments:

#### Maybe can use that abstract a delivery server for MQ message  idempotent.

#### Make an upgrade plan for smooth updates in the future, Such as stop serve  back-up ` unfinished task`  then up new version serve load task.bak, Runing.