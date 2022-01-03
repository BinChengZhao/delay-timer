macro_rules! cfg_status_report {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "status-report")]
            #[cfg_attr(docsrs, doc(cfg(feature = "status-report")))]
            $item
        )*
    };

    ($($item:stmt)*) => {
        $(
            #[cfg(feature = "status-report")]
            $item
        )*
    }
}
