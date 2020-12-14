macro_rules! cfg_tokio_support {
    ($($item:item)*) => {
        $(
            // As long as the features contains tokio-support, it will compile.
            #[cfg(feature = "tokio-support")]

            // it can be compile when .toml `rustdoc-args = ["--cfg", "docsrs"]`,
            // and features contains  tokio-support.
            #[cfg_attr(docsrs, doc(cfg(feature = "tokio-support")))]
            $item
        )*
    }
}

macro_rules! cfg_smol_support {
    ($($item:item)*) => {
        $(
            // features "smol-support" or "smol-support, status-report" can compile
            // features  "smol-support, tokio-support" or "smol-support, tokio-full" can't compile.
#[cfg(not(any(feature = "tokio-support", feature = "tokio-full")))]
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
