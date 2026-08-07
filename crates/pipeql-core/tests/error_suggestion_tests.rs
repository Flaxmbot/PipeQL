#[cfg(test)]
mod tests {
    use pipeql_core::parser::{suggest_keyword, Parser};

    #[test]
    fn test_suggest_keyword() {
        assert_eq!(suggest_keyword("flter"), Some("filter"));
        assert_eq!(suggest_keyword("selet"), Some("select"));
        assert_eq!(suggest_keyword("jion"), Some("join"));
        assert_eq!(suggest_keyword("upsrt"), Some("upsert"));
        assert_eq!(suggest_keyword("frum"), Some("from"));
        assert_eq!(suggest_keyword("tabl"), Some("table"));
    }

    #[test]
    fn test_typo_error_message() {
        let mut parser = Parser::new("from users | flter age >= 18").unwrap();
        let errs = parser.parse_pipeline().unwrap_err();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].message.contains("Unknown pipeline step 'flter'"));
        assert_eq!(errs[0].suggestion.as_deref(), Some("Did you mean `filter`?"));
    }
}
