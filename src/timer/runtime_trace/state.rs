pub(crate) mod instance {
    /// Set if the task is running.
    pub(crate) const RUNNING: usize = 1 << 1;

    /// Set if the task has been completed.
    pub(crate) const COMPLETED: usize = 1 << 2;

    /// Set if the task has been Cancelled.
    pub(crate) const CANCELLED: usize = 1 << 3;
}

pub(crate) mod instance_chain {
    /// Set if the TaskInstancesChain is Living.
    pub(crate) const LIVING: usize = 1 << 1;

    /// Set if the TaskInstancesChain has been dropped.
    pub(crate) const DROPPED: usize = 1 << 2;
}
