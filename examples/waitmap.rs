fn main() {
    use waitmap::WaitMap;

    let mut w: WaitMap<u32, u32> = WaitMap::new();

    let mut zero_mut = w.get_mut(&0);
    //https://www.infoq.cn/article/2017/11/rust-1.22-released
    if zero_mut.is_none() {
        println!("none");

        w.insert(0, 0);
        zero_mut = w.get_mut(&0);
    }

    // dbg!(zero_mut.get_or_insert(1));
}
