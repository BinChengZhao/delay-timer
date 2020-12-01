macro_rules! cfg_tokio_support {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "tokio-support")]
            $item
        )*
    }
}

macro_rules! cfg_smol_support {
    ($($item:item)*) => {
        $(
            //FIXME: any tokio-full and tokio-support
#[cfg(not(feature = "tokio-support"))]
#[cfg(feature = "smol-support")]
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
