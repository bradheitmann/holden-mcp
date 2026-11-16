fn main() {
    println!("declared-app: public command available");
}

#[cfg(test)]
mod tests {
    #[test]
    fn public_command_is_available() {
        assert!(true);
    }
}
