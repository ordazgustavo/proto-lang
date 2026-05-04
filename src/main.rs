use lexer::Lexer;

fn main() {
    let source = include_str!("../assets/kitchen_synk.pr");
    let lexer = Lexer::new(source);
    for token in lexer {
        println!("{token:?}");
    }
}
