// fn main() {
//     use std::fmt::Display;
//     trait Trait {}
//     impl Trait for i32 {}
//     fn returns_a_trait_object() -> impl Trait + Display {
//         5
//     }

//     println!("{}", returns_a_trait_object());

//     let x: i32 = "5".parse().unwrap();
//     let x: u64 = "5".parse().unwrap();

//     pub trait MyParse {
//         type Error;
//         fn parse(s: &str) -> Result<Self, Self::Error>
//         where
//             Self: Sized;
//     }

//     impl MyParse for i32 {
//         type Error = i32;
//         fn parse(_s: &str) -> Result<Self, Self::Error>
//         where
//             Self: Sized,
//         {
//             Ok(5)
//         }
//     }

//     fn my_parse<F: MyParse>(s: &str) -> Result<F, F::Error> {
//         F::parse(s)
//     }

//     let a: i32 = my_parse("1").unwrap();

//     let b = my_parse::<i32>("1").unwrap();

//     println!("MyParse-Success:{}, {}", a, b);

//     fn input_generic<T: Display>(t: T) {
//         println!("input_generic:{}", t);
//     }

//     input_generic(1);

//     input_generic("str-ei");

//     input_generic::<bool>(true);

//     use std::rc::Rc;
//     println!("{}", std::mem::size_of::<&u8>()); // 8
//                                                 // println!("{}", std::mem::size_of::<[u8]>()); // compiler error
//     println!("{}", std::mem::size_of::<&[u8]>()); // 16
//     println!("{}", std::mem::size_of::<&mut [u8]>()); // 16
//     println!("{}", std::mem::size_of::<*mut [u8]>()); // 16
//     println!("{}", std::mem::size_of::<*const [u8]>()); // 16
//     println!("{}", std::mem::size_of::<Box<[u8]>>()); // 16
//     println!("{}", std::mem::size_of::<Rc<[u8]>>()); // 16
// }
use async_std;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
struct Stat {
    count: i32,
}

#[async_std::main]
async fn main() -> () {
    test_iter();
    test_stream().await;
}

async fn test_stream() {
    use futures::stream::{self, StreamExt};

    let mut stat = Arc::new(Mutex::new(Stat { count: 0 }));
    let stat_ref = &stat;

    //尝试下：cargo-cache && cargo-audit
    //https://github.com/RustSec/cargo-audit
    //https://github.com/matthiaskrgr/cargo-cache

    //TODO: 有意义的实战--待记录笔记
    //启示录：https://doc.rust-lang.org/std/sync/struct.Arc.html#examples
    /**
    * 因为 async 有move加持，内部通过 stat绑定.clone  会给 stat本体move进入，外部也无法打印

    如果同步外部提前安置一个引用，然后内部使用这个引用进行clone，会把这个引用move进去，绑定不受影响。

    通过Arc::clone 进行 引用计数，mutex.lock拿出守护的数据进行改变

    外部也能打印
         */
    let result = stream::iter(0..8)
        .then(|i| async move {
            //error, because will move it . outside cat use.
            // let statbak = stat.clone();

            let statbak = Arc::clone(stat_ref);
            let mut real_stat = statbak.lock().unwrap();
            real_stat.count += 1;
            i * i
        })
        .collect::<Vec<i32>>()
        .await;
    dbg!(&result);
    dbg!(&stat);
}

fn test_iter() {
    let mut stat: Stat = Stat { count: 0 };

    let result = (0..8)
        .into_iter()
        .map(|i| {
            stat.count += 1; // <-- 同步班的 iter 是没问题的
            i * i
        })
        .collect::<Vec<i32>>();
    dbg!(&result);
    dbg!(&stat);
}
