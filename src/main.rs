fn main() {
    use std::collections::VecDeque;

    let mut v = VecDeque::with_capacity(3600);
    v.push_back(1);
    v.push_back(2);
    v.push_back(3);

    let mut j = 0;
    loop {
        if j == 5 {
            break;
        }

        for i in v.iter() {
            println!("{}", i);
        }

        j += 1;
    }
}
