//! libfiredancer-core

pub fn hello() -> &'static str {
    "Hello from libfiredancer-core!"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {
        assert_eq!(hello(), "Hello from libfiredancer-core!");
    }
}
