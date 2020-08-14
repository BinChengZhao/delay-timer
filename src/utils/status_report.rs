// status_report is mod  for report node heathy
// if open feature status-report, then compile that mod .
// mapping
// #[cfg(feature="status-report")]
// mod status_report{}
// pub struct statusReporter {

// }

struct Survival {}

struct T<F> {
    survival: Survival,
    report_fn: F,
}
#[cfg(feature = "status-report")]
pub trait statusReport: Send + Sync + 'static {
    type situation = Result<bool>;

    // new a delaytimer::Task to run it....!

    //
    ///
    /// ```
    /// let example_task  = Task::spawn( ||{
    ///         let result = report.report().await;
    ///
    ///         if result.is_err() {
    ///            report.help().await;
    ///         }
    ///
    /// } ).detach();
    ///
    /// ```
    async fn report(&mut self, t: T) -> Self::situation {

        // t is alies of LinkedList<record> or Vec<record> or ...T<record>
    }

    // if report error or world destory... call help ..... call user....
    async fn help() {}
}
