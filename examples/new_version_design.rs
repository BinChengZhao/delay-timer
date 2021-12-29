use std::future::Future;


// For Async Task
struct NewAsyncFn<F: Fn() -> U, U: Future>(F);
// For Sync Task
struct NewSyncFn<F: Fn() -> ()>(F);
// Task Context
struct Context;
// Task Collector
struct Collector{
    inner:Vec<Box<dyn Operator <Handle=i32>>>
}


// Task Abstract
trait Operator {
    type Handle;
    fn spawn(&self) -> Self::Handle;
}


impl<F: Fn() -> U, U: Future> Operator for NewAsyncFn<F, U> {
    type Handle = i32;
    fn spawn(&self) -> Self::Handle {
        let user_future = (&self.0)();
        let context = Context;
        
        let task = async {
            user_future.await;
           // context.do_someting();
        };
        
        // let handle = spawn(task);
        // return handle

        1
    }
}


impl<F: Fn()> Operator for NewSyncFn<F> {
    type Handle = u64;
    fn spawn(&self) -> Self::Handle {
        // do someting
        1
    }
}


#[tokio::main]
async fn main() {
    wrap_contex(|| async move {
        println!("haha");
    })
    .await;
}


// like `impl Fn -> impl Future`
fn wrap_contex<T, F>(f: T) -> impl Future
where
    T: Fn() -> F,
    F: Future<Output = ()>,
{
    async move {
        println!("before");
        f().await;
        println!("after");
    }
}