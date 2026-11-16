fn main() {
    println!("declared-tool-b: public command available");
}

#[cfg(test)]
mod tests {
    #[test]
    fn tool_b_runs() {
        assert_eq!(3 + 3, 6);
    }
}
