fn main() {
    println!("declared-tool-a: public command available");
}

#[cfg(test)]
mod tests {
    #[test]
    fn tool_a_runs() {
        assert_eq!(2 + 2, 4);
    }
}
