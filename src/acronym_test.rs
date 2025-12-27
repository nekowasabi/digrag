#[cfg(test)]
mod acronym_tests {
    use lindera::{
        dictionary::{load_embedded_dictionary, DictionaryKind},
        mode::Mode,
        segmenter::Segmenter,
        tokenizer::Tokenizer,
    };

    #[test]
    fn test_english_acronyms() {
        let dictionary = load_embedded_dictionary(DictionaryKind::IPADIC).unwrap();
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        let tokenizer = Tokenizer::new(segmenter);
        
        let test_cases = vec![
            ("MCP", "English acronym alone"),
            ("API", "English acronym alone"),
            ("LLM", "English acronym alone"),
            ("SDK", "English acronym alone"),
            ("MCPサーバー", "Acronym with Japanese"),
            ("API連携", "Acronym with Japanese"),
            ("LLMを使う", "Acronym with Japanese"),
            ("MCP API SDK LLM", "Multiple acronyms"),
        ];
        
        for (text, desc) in test_cases {
            let mut tokens = tokenizer.tokenize(text).unwrap();
            println!("=== {} ===", desc);
            println!("Input: {}", text);
            for token in tokens.iter_mut() {
                let details = token.details();
                println!("  Token: '{}' -> POS: {:?}", token.surface, details);
            }
            println!();
        }
    }
}
