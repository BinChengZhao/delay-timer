fn main() {
    use std::fmt::Display;
    trait Trait {}
    impl Trait for i32 {}
    fn returns_a_trait_object() -> impl Trait + Display {
        5
    }

    println!("{}", returns_a_trait_object());

    let x: i32 = "5".parse().unwrap();
    let x: u64 = "5".parse().unwrap();

    pub trait MyParse {
        type Error;
        fn parse(s: &str) -> Result<Self, Self::Error>
        where
            Self: Sized;
    }

    impl MyParse for i32 {
        type Error = i32;
        fn parse(_s: &str) -> Result<Self, Self::Error>
        where
            Self: Sized,
        {
            Ok(5)
        }
    }

    fn my_parse<F: MyParse>(s: &str) -> Result<F, F::Error> {
        F::parse(s)
    }

    let a: i32 = my_parse("1").unwrap();

    let b = my_parse::<i32>("1").unwrap();

    println!("MyParse-Success:{}, {}", a, b);

    fn input_generic<T: Display>(t: T) {
        println!("input_generic:{}", t);
    }

    input_generic(1);

    input_generic("str-ei");

    input_generic::<bool>(true);

    use std::rc::Rc;
    println!("{}", std::mem::size_of::<&u8>()); // 8
                                                // println!("{}", std::mem::size_of::<[u8]>()); // compiler error
    println!("{}", std::mem::size_of::<&[u8]>()); // 16
    println!("{}", std::mem::size_of::<&mut [u8]>()); // 16
    println!("{}", std::mem::size_of::<*mut [u8]>()); // 16
    println!("{}", std::mem::size_of::<*const [u8]>()); // 16
    println!("{}", std::mem::size_of::<Box<[u8]>>()); // 16
    println!("{}", std::mem::size_of::<Rc<[u8]>>()); // 16
}
