// cpu-enough: harness entry point (1.1 scaffold; subcommands land with the 1.2 modules).

fn main() {
    println!("cpu-enough {}", env!("CARGO_PKG_VERSION"));
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_set() {
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
    }
}
