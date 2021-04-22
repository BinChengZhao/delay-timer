/// State of the task instance.
pub mod instance {
    /// Set if the task is running.
    pub const RUNNING: usize = 1 << 1;

    /// Set if the task has been completed.
    pub const COMPLETED: usize = 1 << 2;

    /// Set if the task has been Cancelled.
    pub const CANCELLED: usize = 1 << 3;

    /// Set if the task has been Timeout.
    pub const TIMEOUT: usize = 1 << 4;
}

pub(crate) mod instance_chain {
    /// Set if the TaskInstancesChain is Living.
    pub(crate) const LIVING: usize = 1 << 1;

    /// Set if the TaskInstancesChain has been dropped.
    pub(crate) const DROPPED: usize = 1 << 2;

    /// Set if the `TaskInstancesChainMaintainer` has been dropped.
    /// Indicates that the running instance of the task is no longer maintained.
    pub(crate) const ABANDONED: usize = 1 << 3;
}
