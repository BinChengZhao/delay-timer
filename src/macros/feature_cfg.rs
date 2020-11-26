macro_rules! cfg_tokio_support {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "tokio-support")]
            $item
        )*
    }
}

macro_rules! cfg_not_tokio_support {
    ($($item:item)*) => {
        $(
#[cfg(not(feature = "tokio-support"))]
            $item
        )*
    }
}

macro_rules! cfg_status_report {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "status-report")]
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
