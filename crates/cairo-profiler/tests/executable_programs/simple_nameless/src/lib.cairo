fn do_add() -> u8 {
    let sum = 2 + 2;
    sum
}

#[executable]
fn main() {
    let sum = do_add();
    println!("{}", sum);
}
