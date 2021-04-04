pub(crate) mod instance {
    /// Set if `TaskInstancesChain` currently exists.
    pub(crate) const CHAIN: usize = 1 << 0;

    /// Set if `InstanceList` currently exists.
    pub(crate) const LIST: usize = 1 << 1;
}
