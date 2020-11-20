// status_report is mod  for report node heathy
// if open feature status-report, then compile that mod .
// mapping
use smol::channel::{Receiver as AsyncReceiver, Sender as AsyncSender};
cfg_status_report!(

use async_trait::async_trait;

    #[async_trait]
    pub trait StatusReport: Send + Sync + 'static {
        type Situation = Result<Self::Normal, Self::Exception>;
        type Normal = bool;
        type Exception = String;
    
        // type
    
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
        //TODO: use async Trait.
        async fn report(&mut self, t: AsyncReceiver<i32>) -> Self::situation {
    
            // t is alies of LinkedList<record> or Vec<record> or ...T<record>
        }
    
        // if report error or world destory... call help ..... call user....
        async fn help(&mut self, expression: Self::Exception) {}
    }
    
);