# delayTimer
Time-manager of delayed tasks.


## To Do List
- [x] async-channel for timer_core.
- [x] separate fn handle_event in timer and have no lock ,maybe can remove async-mutex.
- [x] add update-event ,handle_event can use it msg for update timer_core, update-event(task_id, old_slotid, new_slotid, call_at) handle_event can use call_at check if postpone run.
- [x] when append that generate async-wrapper macro, ready for release.
- [x] append that generate bloking-wrapper macro, ready for release.
- [x] batch-opration.
- [x] report-for-server.
- [x] TASK-TAG.