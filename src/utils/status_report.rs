// status_report is mod  for report node heathy
// if open feature status-report, then compile that mod .
// mapping
use smol::channel::{Receiver as AsyncReceiver, Sender as AsyncSender};
use std::sync::Arc;
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
        async fn report(self:Arc<Self>, _t: Option<AsyncReceiver<i32>>) -> Self::Situation {
            // Ok(true)
           todo!();
            // t is alies of LinkedList<record> or Vec<record> or ...T<record>
        }
    
        // if report error or world destory... call help ..... call user....
        //Self::Exception
        async fn help(self:Arc<Self>, expression: String) {}
    }
    
);