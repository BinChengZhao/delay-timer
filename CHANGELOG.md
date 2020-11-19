# Version 0.2.0

- Add `tokio-support` and `status-report`  features, support for tokio ecology, internal logic optimization, generate tasks faster, add syntactic sugar to cron-expressions, etc.

# Version 0.1.0

- delay-timer is a task manager based on a time wheel algorithm, which makes it easy to manage timed tasks, or to periodically execute arbitrary tasks such as closures.

The underlying runtime is currently based on smol, so upper level applications that want to extend asynchronous functionality need to use libraries that are compatible with smol.

Since the library currently includes features such as #[bench], it needs to be developed in a nightly version.
