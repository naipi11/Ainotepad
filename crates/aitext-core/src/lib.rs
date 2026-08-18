pub fn workspace_name() -> &'static str {
    "aitext"
}

#[cfg(test)]
mod tests {
    use super::workspace_name;

    #[test]
    fn workspace_name_is_aitext() {
        assert_eq!(workspace_name(), "aitext");
    }
}
